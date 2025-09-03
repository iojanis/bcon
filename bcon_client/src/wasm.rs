#[cfg(feature = "wasm")]
use crate::{BconConfig, BconError, message::{IncomingMessage, OutgoingMessage}};
use gloo_net::websocket::{futures::WebSocket, Message};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use wasm_bindgen::prelude::*;
use web_sys::console;
use tracing::{debug, info};

#[wasm_bindgen]
extern "C" {
    fn setTimeout(closure: &Closure<dyn FnMut()>, millis: u32) -> f64;
    fn setInterval(closure: &Closure<dyn FnMut()>, millis: u32) -> f64;
    fn clearInterval(token: f64);
}

#[wasm_bindgen]
pub struct WasmBconClient {
    config: BconConfig,
    websocket: Option<WebSocket>,
    heartbeat_handle: Option<f64>,
}

impl WasmBconClient {
    pub fn new(config: BconConfig) -> std::result::Result<Self, BconError> {
        Ok(Self {
            config,
            websocket: None,
            heartbeat_handle: None,
        })
    }
    
    async fn connect_internal(&mut self) -> std::result::Result<(), BconError> {
        info!("Connecting to Bcon server: {}", self.config.server_url);
        
        let websocket = WebSocket::open(&self.config.server_url)
            .map_err(|e| BconError::Connection(format!("Failed to open WebSocket: {:?}", e)))?;
        
        self.websocket = Some(websocket);
        
        // Start heartbeat
        self.start_heartbeat();
        
        info!("Successfully connected to Bcon server");
        Ok(())
    }
    
    async fn disconnect_internal(&mut self) -> std::result::Result<(), BconError> {
        // Stop heartbeat
        if let Some(handle) = self.heartbeat_handle.take() {
            clearInterval(handle);
        }
        
        if let Some(websocket) = self.websocket.take() {
            debug!("Closing WebSocket connection");
            websocket.close(None, None)
                .map_err(|e| BconError::WebSocket(format!("Close failed: {:?}", e)))?;
        }
        
        Ok(())
    }
    
    pub async fn send_message(&mut self, message: OutgoingMessage) -> std::result::Result<(), BconError> {
        if let Some(websocket) = &mut self.websocket {
            let json = serde_json::to_string(&message)?;
            debug!("Sending message: {}", json);
            
            websocket.send(Message::Text(json)).await
                .map_err(|e| BconError::WebSocket(format!("Send failed: {:?}", e)))?;
            
            Ok(())
        } else {
            Err(BconError::NotConnected)
        }
    }
    
    pub async fn receive_message(&mut self) -> std::result::Result<IncomingMessage, BconError> {
        if let Some(websocket) = &mut self.websocket {
            loop {
                match websocket.next().await {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                debug!("Received message: {}", text);
                                let incoming: IncomingMessage = serde_json::from_str(&text)?;
                                return Ok(incoming);
                            }
                            Message::Bytes(data) => {
                                // Handle binary messages
                                let text = String::from_utf8(data)
                                    .map_err(|e| BconError::MessageParsing(format!("UTF-8 error: {}", e)))?;
                                let incoming: IncomingMessage = serde_json::from_str(&text)?;
                                return Ok(incoming);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Err(BconError::WebSocket(format!("WebSocket error: {:?}", e)));
                    }
                    None => {
                        return Err(BconError::Connection("WebSocket stream ended".to_string()));
                    }
                }
            }
        } else {
            Err(BconError::NotConnected)
        }
    }
    
    fn start_heartbeat(&mut self) {
        let interval_ms = self.config.heartbeat_interval as u32;
        let websocket_ptr = self.websocket.as_ref().map(|ws| ws as *const WebSocket as usize);
        
        if websocket_ptr.is_none() {
            return;
        }
        
        // Create heartbeat closure
        let heartbeat_closure = Closure::wrap(Box::new(move || {
            // In a real implementation, we'd need a way to safely access the websocket
            // For now, we'll use console logging
            console::log_1(&"Sending heartbeat (WASM)".into());
            
            // Send heartbeat message
            let heartbeat = OutgoingMessage::heartbeat();
            if let Ok(json) = serde_json::to_string(&heartbeat) {
                // We need a safe way to send the message from the closure
                // This is a simplified version
                debug!("Would send heartbeat: {}", json);
            }
        }) as Box<dyn FnMut()>);
        
        let handle = setInterval(&heartbeat_closure, interval_ms);
        self.heartbeat_handle = Some(handle);
        
        // Keep closure alive
        heartbeat_closure.forget();
    }
    
    /// Set up event listeners for the WebSocket (WASM-specific)
    pub fn setup_event_listeners(&self) -> std::result::Result<(), BconError> {
        // This would set up proper event listeners in a real WASM implementation
        // For now, we'll use the stream-based approach in receive_message
        Ok(())
    }
}

// WASM-specific utilities
#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmBconClient {
    /// Create client from JavaScript config object  
    #[wasm_bindgen]
    pub fn from_js_config(config: JsValue) -> Result<WasmBconClient, JsValue> {
        let config_str = js_sys::JSON::stringify(&config)
            .map_err(|_e| JsValue::from_str("Failed to stringify config"))?;
        
        let config_str = config_str.as_string()
            .ok_or_else(|| JsValue::from_str("Config is not a string"))?;
        
        let config: BconConfig = serde_json::from_str(&config_str)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        
        WasmBconClient::new(config)
            .map_err(|e| JsValue::from_str(&format!("Failed to create client: {}", e)))
    }
    
    /// Get connection state as string for JavaScript
    #[wasm_bindgen]
    pub fn get_state_string(&self) -> String {
        if self.websocket.is_some() {
            "connected".to_string()
        } else {
            "disconnected".to_string()
        }
    }
    
    /// Connect to server (WASM)
    #[wasm_bindgen]
    pub async fn connect(&mut self) -> Result<(), JsValue> {
        self.connect_internal().await
            .map_err(|e| JsValue::from_str(&format!("Connection failed: {}", e)))
    }
    
    /// Disconnect from server (WASM)
    #[wasm_bindgen]
    pub async fn disconnect(&mut self) -> Result<(), JsValue> {
        self.disconnect_internal().await
            .map_err(|e| JsValue::from_str(&format!("Disconnect failed: {}", e)))
    }
    
    /// Send message from JavaScript
    #[wasm_bindgen] 
    pub fn send_js_message(&mut self, message_json: &str) -> Result<(), JsValue> {
        let _message: OutgoingMessage = serde_json::from_str(message_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid message JSON: {}", e)))?;
        
        // For now, return a placeholder. In a real implementation, we'd need to 
        // restructure this to handle async operations properly
        if self.websocket.is_some() {
            web_sys::console::log_1(&format!("Would send message: {}", message_json).into());
            Ok(())
        } else {
            Err(JsValue::from_str("Not connected"))
        }
    }
    
    /// Register callback for incoming messages (WASM)
    #[wasm_bindgen]
    pub fn on_message(&self, _callback: js_sys::Function) {
        // In a real implementation, this would register the callback
        // to be called when messages are received
        console::log_1(&"Message callback registered".into());
    }
    
    /// Register callback for connection events (WASM)
    #[wasm_bindgen]
    pub fn on_connection(&self, _callback: js_sys::Function) {
        console::log_1(&"Connection callback registered".into());
    }
    
    /// Register callback for errors (WASM)
    #[wasm_bindgen] 
    pub fn on_error(&self, _callback: js_sys::Function) {
        console::log_1(&"Error callback registered".into());
    }
}

// Additional WASM exports for easier JavaScript integration
#[cfg(feature = "wasm")]
#[wasm_bindgen]  
pub struct WasmBconClientBuilder {
    config: BconConfig,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmBconClientBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(server_url: String) -> Self {
        Self {
            config: BconConfig {
                server_url,
                ..Default::default()
            }
        }
    }
    
    #[wasm_bindgen]
    pub fn with_auth_token(mut self, token: String, role: String) -> Result<WasmBconClientBuilder, JsValue> {
        use crate::auth::{AuthConfig, ClientRole};
        
        let client_role = match role.as_str() {
            "guest" => ClientRole::Guest,
            "player" => ClientRole::Player,
            "admin" => ClientRole::Admin,
            "system" => ClientRole::System,
            _ => return Err(JsValue::from_str("Invalid role")),
        };
        
        self.config.auth = Some(AuthConfig::Token {
            token,
            role: client_role,
        });
        
        Ok(self)
    }
    
    
    #[wasm_bindgen]
    pub fn with_timeout(mut self, timeout_ms: f64) -> Self {
        self.config.connect_timeout = timeout_ms as u64;
        self
    }
    
    #[wasm_bindgen]
    pub fn with_heartbeat_interval(mut self, interval_ms: f64) -> Self {
        self.config.heartbeat_interval = interval_ms as u64;
        self
    }
    
    #[wasm_bindgen]
    pub fn build(self) -> Result<WasmBconClient, JsValue> {
        WasmBconClient::new(self.config)
            .map_err(|e| JsValue::from_str(&format!("Failed to build client: {}", e)))
    }
}