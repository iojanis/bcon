use bcon_client::{
    BconClient, BconConfig, BconEventHandler, BconError,
    auth::AuthConfig,
    message::{IncomingMessage, OutgoingMessage},
    ClientInfo,
};
use serde_json;
use std::env;
use tracing::{info, warn, error};

/// Example system client that receives all adapter events and can send commands
struct SystemEventHandler;

impl BconEventHandler for SystemEventHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        info!("🔌 Connected as system client");
        info!("   Connection ID: {}", client_info.connection_id);
        info!("   Role: {:?}", client_info.role);
    }
    
    fn on_disconnected(&mut self, reason: String) {
        warn!("❌ Disconnected: {}", reason);
    }
    
    fn on_message(&mut self, message: IncomingMessage) {
        info!("📥 Received: {}", message.message_type);
        
        // Check if this is a relay message from an adapter
        if message.is_from_adapter() {
            if let Ok(relay_message) = message.extract_relay_message() {
                info!("🔌 Adapter Event:");
                info!("   Server: {:?}", relay_message.source_id);
                info!("   Type: {}", relay_message.message_type);
                
                // Example: Handle specific event types
                match relay_message.message_type.as_str() {
                    "player_joined" => {
                        info!("👋 Player joined event received");
                        // Could forward to web clients, log to database, etc.
                    }
                    "player_left" => {
                        info!("👋 Player left event received");
                    }
                    "chat_message" => {
                        info!("💬 Chat message received");
                        // Could implement chat filtering, logging, etc.
                    }
                    "server_started" => {
                        info!("🚀 Server started");
                    }
                    _ => {
                        info!("ℹ️  Other event: {}", relay_message.message_type);
                    }
                }
            }
        } else {
            info!("📋 Server message: {}", message.message_type);
        }
    }
    
    fn on_error(&mut self, error: BconError) {
        error!("❌ Error: {}", error);
    }
    
    fn on_auth_failed(&mut self, reason: String) {
        error!("🔐 Authentication failed: {}", reason);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Get token from environment or command line
    let token = env::args().nth(1)
        .or_else(|| env::var("BCON_SYSTEM_TOKEN").ok())
        .expect("Please provide system token as argument or BCON_SYSTEM_TOKEN environment variable");
    
    let server_url = env::var("BCON_SERVER_URL")
        .unwrap_or_else(|_| "ws://localhost:8081".to_string());
    
    info!("🔧 Starting system client");
    info!("   Server: {}", server_url);
    
    // Create system client configuration
    let config = BconConfig::system(server_url, token);
    let mut client = BconClient::new(config);
    
    // Connect to server
    info!("🔌 Connecting to Bcon server...");
    client.connect().await?;
    
    // Example: Send a command to an adapter after connecting
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    info!("📤 Sending test command to adapters");
    let test_command_data = serde_json::json!({
        "action": "get_server_status"
    });
    
    if let Err(e) = client.send_adapter_command(
        None, // Send to all adapters
        "status_request".to_string(),
        test_command_data,
    ).await {
        error!("Failed to send adapter command: {}", e);
    }
    
    // Start event loop to receive messages
    let handler = SystemEventHandler;
    info!("🎧 Starting event loop...");
    client.start_event_loop(handler).await?;
    
    Ok(())
}