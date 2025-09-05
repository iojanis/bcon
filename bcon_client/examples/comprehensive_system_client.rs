use bcon_client::{
    BconClient, BconConfig, BconEventHandler, BconError,
    message::{IncomingMessage, OutgoingMessage},
    ClientInfo,
};
use serde_json;
use std::env;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn, error, debug};

/// Comprehensive system client that handles all client messages and adapter events
struct ComprehensiveSystemHandler;

impl ComprehensiveSystemHandler {
    /// Handle chat messages from players/guests/admins
    fn handle_chat_message(&self, message: &IncomingMessage, client_id: &str, client_role: &str) {
        info!("💬 Processing chat message from {} ({})", client_id, client_role);
        
        if let Some(chat_text) = message.data.get("message").and_then(|v| v.as_str()) {
            info!("   Message: {}", chat_text);
            
            // Example: Process different chat commands
            if chat_text.starts_with("!ping") {
                info!("   🏓 Ping command detected - would respond with pong");
            } else if chat_text.starts_with("!time") {
                info!("   ⏰ Time command detected - would respond with current time");
            } else if chat_text.starts_with("!help") {
                info!("   ❓ Help command detected - would respond with command list");
            } else if chat_text.starts_with("!") {
                info!("   🔧 Unknown command - would send error response");
            } else {
                info!("   💬 Regular chat message - would broadcast or log");
            }
        }
    }

    /// Handle command execution requests from admins/system clients  
    fn handle_execute_command(&self, message: &IncomingMessage, client_id: &str, client_role: &str) {
        info!("⚡ Processing command from {} ({})", client_id, client_role);
        
        if let Some(command) = message.data.get("command").and_then(|v| v.as_str()) {
            info!("   Command: {}", command);
            
            // Example: Handle some commands locally, others would be forwarded to adapters
            match command {
                "status" | "uptime" | "help" => {
                    info!("   📊 System command - would respond with local status");
                }
                _ => {
                    info!("   🔄 Minecraft command - would forward to adapters for execution");
                }
            }
        }
    }

    /// Handle server info requests
    fn handle_server_info(&self, _message: &IncomingMessage, client_id: &str) -> Option<serde_json::Value> {
        info!("ℹ️  Processing server info request from {}", client_id);
        
        Some(serde_json::json!({
            "success": true,
            "result": {
                "system_client": "comprehensive_system_client",
                "version": "1.0.0",
                "capabilities": ["chat_processing", "command_execution", "adapter_forwarding"],
                "status": "active",
                "uptime": "running"
            },
            "processed_by": "comprehensive_system_client"
        }))
    }

    /// Handle messages from other clients (guests, players, admins)  
    /// Returns a response that should be sent back to the client
    fn handle_client_message(&self, message: &IncomingMessage) -> Option<OutgoingMessage> {
        // Extract client context that was added by the server's message router
        let client_id = message.data.get("client_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let client_role = message.data.get("client_role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        info!("📨 Handling client message: {} from {} ({})", message.message_type, client_id, client_role);

        let response_data = match message.message_type.as_str() {
            "send_chat" => {
                self.handle_chat_message(message, client_id, client_role);
                // For chat, return success response
                Some(serde_json::json!({
                    "success": true,
                    "result": "Chat message processed by system client",
                    "processed_by": "comprehensive_system_client"
                }))
            }
            "execute_command" => {
                self.handle_execute_command(message, client_id, client_role);
                // For commands, return success response  
                Some(serde_json::json!({
                    "success": true,
                    "result": "Command processed by system client",
                    "processed_by": "comprehensive_system_client"
                }))
            }
            "get_server_info" => self.handle_server_info(message, client_id),
            _ => {
                info!("🔄 Unhandled client message type: {}", message.message_type);
                Some(serde_json::json!({
                    "success": false,
                    "error": format!("Unhandled message type: {}", message.message_type),
                    "processed_by": "comprehensive_system_client"
                }))
            }
        };

        // Create response message if we have response data
        if let (Some(data), Some(reply_to)) = (response_data, &message.message_id) {
            Some(OutgoingMessage::ack_success(reply_to.clone(), data))
        } else {
            None
        }
    }

    /// Handle messages from adapters
    fn handle_adapter_message(&self, message: &IncomingMessage) {
        if let Ok(relay_message) = message.extract_relay_message() {
            info!("🔌 Adapter Event from {:?}:", relay_message.source_id);
            info!("   Type: {}", relay_message.message_type);
            
            // Handle specific adapter events
            match relay_message.message_type.as_str() {
                "player_joined" => {
                    if let Some(player_name) = relay_message.data.get("playerName") {
                        info!("👋 Player {} joined server", player_name);
                    }
                }
                "player_left" => {
                    if let Some(player_name) = relay_message.data.get("playerName") {
                        info!("👋 Player {} left server", player_name);
                    }
                }
                "chat_message" => {
                    info!("💬 Chat message from adapter");
                }
                "server_started" => {
                    info!("🚀 Server started");
                }
                "server_stopped" => {
                    info!("🛑 Server stopped");
                }
                "command_result" => {
                    info!("📊 Command execution result from adapter");
                    if let Ok(pretty) = serde_json::to_string_pretty(&relay_message.data) {
                        info!("Result: {}", pretty);
                    }
                }
                _ => {
                    debug!("ℹ️  Other adapter event: {}", relay_message.message_type);
                }
            }
        }
    }
}

/// Event handler implementation with response channel
pub struct SystemClientEventHandler {
    handler: ComprehensiveSystemHandler,
    response_sender: mpsc::UnboundedSender<OutgoingMessage>,
}

impl SystemClientEventHandler {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<OutgoingMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self {
            handler: ComprehensiveSystemHandler,
            response_sender: sender,
        }, receiver)
    }
}

impl BconEventHandler for SystemClientEventHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        info!("✅ COMPREHENSIVE SYSTEM CLIENT CONNECTED");
        info!("   Connection ID: {}", client_info.connection_id);
        info!("   Role: {:?}", client_info.role);
        info!("   🎯 Ready to process messages from guests, players, and admins");
        info!("   🔌 Adapter forwarding enabled");
        info!("   📤 Response functionality active");
    }
    
    fn on_disconnected(&mut self, reason: String) {
        warn!("❌ System client disconnected: {}", reason);
    }
    
    fn on_message(&mut self, message: IncomingMessage) {
        debug!("📥 Received message: {}", message.message_type);
        
        if message.is_from_adapter() {
            // Message from adapter - just log and process
            self.handler.handle_adapter_message(&message);
        } else {
            // Message from client (guest/player/admin) - process and send response
            if let Some(response) = self.handler.handle_client_message(&message) {
                if let Err(e) = self.response_sender.send(response) {
                    error!("❌ Failed to queue response: {}", e);
                }
            }
        }
    }
    
    fn on_error(&mut self, error: BconError) {
        error!("❌ System client error: {}", error);
    }
    
    fn on_auth_failed(&mut self, reason: String) {
        error!("🔐 Authentication failed: {}", reason);
    }
}

// Implement Clone for ComprehensiveSystemHandler
impl Clone for ComprehensiveSystemHandler {
    fn clone(&self) -> Self {
        Self
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with timestamps
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Get configuration from environment or command line
    let token = env::args().nth(1)
        .or_else(|| env::var("BCON_SYSTEM_TOKEN").ok())
        .expect("Please provide system token as argument or BCON_SYSTEM_TOKEN environment variable");
    
    let server_url = env::var("BCON_SERVER_URL")
        .unwrap_or_else(|_| "ws://localhost:8081".to_string());
    
    info!("🚀 STARTING COMPREHENSIVE SYSTEM CLIENT");
    info!("   Server: {}", server_url);
    info!("   Capabilities: Client message processing, RCON support, Adapter forwarding");
    
    // Create system client configuration
    let config = BconConfig::system(server_url, token);
    let mut client = BconClient::new(config);
    
    // Connect to server
    info!("🔌 Connecting to Bcon server...");
    client.connect().await?;
    
    // Create handler and response channel
    let (handler, mut response_receiver) = SystemClientEventHandler::new();
    
    // Start event loop to receive and process all messages
    info!("🎧 Starting comprehensive event processing loop...");
    info!("   Now ready to handle:");
    info!("   📱 Guest/Player/Admin messages (chat, commands, info requests)");
    info!("   🔌 Adapter event monitoring and logging");
    info!("   📤 Automatic response generation and sending");
    
    // Clone client for response sending
    let client_arc = Arc::new(Mutex::new(client));
    let response_client = Arc::clone(&client_arc);
    
    // Start response sender task
    tokio::spawn(async move {
        while let Some(response) = response_receiver.recv().await {
            let mut client = response_client.lock().await;
            match client.send_message(response).await {
                Ok(()) => info!("✅ Response sent back to client"),
                Err(e) => error!("❌ Failed to send response to client: {}", e),
            }
        }
    });
    
    // Start main event loop  
    let mut event_client = Arc::try_unwrap(client_arc)
        .map_err(|_| "Failed to get exclusive access to client")?
        .into_inner();
    
    event_client.start_event_loop(handler).await?;
    
    Ok(())
}