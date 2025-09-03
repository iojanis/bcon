#[cfg(feature = "native")]
use crate::{BconConfig, BconError, Result, message::{IncomingMessage, OutgoingMessage}};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use tracing::{debug, error, info};
use std::time::Duration;

pub struct NativeBconClient {
    config: BconConfig,
    websocket: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl NativeBconClient {
    pub fn new(config: BconConfig) -> Result<Self> {
        Ok(Self {
            config,
            websocket: None,
        })
    }
    
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to Bcon server: {}", self.config.server_url);
        
        // Parse URL
        let url = url::Url::parse(&self.config.server_url)
            .map_err(|e| BconError::Configuration(format!("Invalid URL: {}", e)))?;
        
        // Set up connection timeout
        let connect_future = connect_async(&url);
        let timeout = Duration::from_millis(self.config.connect_timeout);
        
        // Connect with timeout
        let (websocket, response) = tokio::time::timeout(timeout, connect_future)
            .await
            .map_err(|_| BconError::Timeout)?
            .map_err(|e| BconError::Connection(format!("WebSocket connection failed: {}", e)))?;
        
        debug!("WebSocket connection established, status: {}", response.status());
        self.websocket = Some(websocket);
        
        info!("Successfully connected to Bcon server");
        Ok(())
    }
    
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut websocket) = self.websocket.take() {
            debug!("Closing WebSocket connection");
            websocket.close(None).await
                .map_err(|e| BconError::WebSocket(format!("Close failed: {}", e)))?;
        }
        Ok(())
    }
    
    pub async fn send_message(&mut self, message: OutgoingMessage) -> Result<()> {
        if let Some(websocket) = &mut self.websocket {
            let json = serde_json::to_string(&message)?;
            debug!("Sending message: {}", json);
            
            let msg = Message::Text(json);
            websocket.send(msg).await
                .map_err(|e| BconError::WebSocket(format!("Send failed: {}", e)))?;
            
            Ok(())
        } else {
            Err(BconError::NotConnected)
        }
    }
    
    pub async fn receive_message(&mut self) -> Result<IncomingMessage> {
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
                            Message::Binary(data) => {
                                // Handle binary messages if needed
                                let text = String::from_utf8(data)
                                    .map_err(|e| BconError::Connection(format!("Invalid UTF-8: {}", e)))?;
                                let incoming: IncomingMessage = serde_json::from_str(&text)?;
                                return Ok(incoming);
                            }
                            Message::Close(close_frame) => {
                                let reason = close_frame
                                    .map(|f| format!("Code: {}, Reason: {}", f.code, f.reason))
                                    .unwrap_or_else(|| "No close frame".to_string());
                                return Err(BconError::Connection(format!("Connection closed: {}", reason)));
                            }
                            Message::Ping(data) => {
                                debug!("Received ping, sending pong");
                                websocket.send(Message::Pong(data)).await
                                    .map_err(|e| BconError::WebSocket(format!("Pong failed: {}", e)))?;
                                continue;
                            }
                            Message::Pong(_) => {
                                debug!("Received pong");
                                continue;
                            }
                            Message::Frame(_) => {
                                // Raw frames, skip
                                continue;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Err(BconError::WebSocket(format!("WebSocket error: {}", e)));
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
    
    // Heartbeat would be handled by the main client, not here
    // This was causing lifetime issues
}