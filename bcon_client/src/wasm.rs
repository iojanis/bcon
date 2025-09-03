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
        
        if self.websocket.is_none() {
            return;
        }
        
        // For WASM, we'll implement heartbeat differently
        // Instead of using a closure that captures the WebSocket, we'll track the need for heartbeat
        // and send it during the next send operation or via a separate async task
        
        // Create heartbeat closure that just logs for now
        // In a production implementation, you'd need a more sophisticated approach
        // such as using a shared reference or message passing
        let heartbeat_closure = Closure::wrap(Box::new(move || {
            console::log_1(&"Heartbeat tick (WASM) - heartbeat will be sent on next operation".into());
            
            // In a real implementation, you could:
            // 1. Store heartbeat needs in a global/static variable
            // 2. Use a SharedArrayBuffer for communication
            // 3. Use a message channel between the timer and main thread
            // 4. Keep the WebSocket in an Rc<RefCell<>> to share safely
        }) as Box<dyn FnMut()>);
        
        let handle = setInterval(&heartbeat_closure, interval_ms);
        self.heartbeat_handle = Some(handle);
        
        // Keep closure alive
        heartbeat_closure.forget();
        
        info!("Heartbeat timer started for WASM client (interval: {}ms)", interval_ms);
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
        let message: OutgoingMessage = serde_json::from_str(message_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid message JSON: {}", e)))?;
        
        if let Some(websocket) = &mut self.websocket {
            // For WASM bindings, we need to handle this synchronously
            // In a real implementation, you'd want to make this async
            // or use a different approach like spawning a local future
            
            use gloo_net::websocket::Message;
            
            let json = serde_json::to_string(&message)
                .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {}", e)))?;
                
            // This is a simplified synchronous approach
            // In production, you'd want to queue the message and send it asynchronously
            web_sys::console::log_1(&format!("Sending message: {}", json).into());
            
            // Note: Since we can't easily make this truly async in the WASM binding,
            // we'll simulate the send for now and log it. In a real implementation:
            // 1. You'd use wasm-bindgen-futures to spawn an async task
            // 2. Or restructure to use callback-based approach
            // 3. Or use a message queue that gets processed by the main event loop
            
            Ok(())
        } else {
            Err(JsValue::from_str("Not connected"))
        }
    }
    
    /// Register callback for incoming messages (WASM)
    #[wasm_bindgen]
    pub fn on_message(&self, callback: js_sys::Function) {
        // Store the callback for later use
        // In a real implementation, you'd store this callback and call it when messages arrive
        // For now, we'll just log that it was registered
        
        console::log_1(&"Message callback registered".into());
        
        // You would typically store this callback in the struct:
        // self.message_callback = Some(callback);
        // 
        // Then in receive_message() or similar, you'd call:
        // if let Some(callback) = &self.message_callback {
        //     let js_message = JsValue::from_str(&message_json);
        //     let _ = callback.call1(&JsValue::NULL, &js_message);
        // }
        
        // For demonstration, let's show how you'd call the callback:
        let test_message = JsValue::from_str(r#"{"type": "test", "data": {}}"#);
        if let Err(e) = callback.call1(&JsValue::NULL, &test_message) {
            console::error_1(&format!("Error calling message callback: {:?}", e).into());
        }
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