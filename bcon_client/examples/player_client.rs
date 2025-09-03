use bcon_client::{
    BconClient, BconConfig, BconEventHandler, BconError,
    auth::AuthConfig,
    message::{IncomingMessage, OutgoingMessage},
    ClientInfo,
};
use std::env;
use tracing::{info, warn, error};

/// Example player client that can send chat messages and receive responses
struct PlayerEventHandler;

impl BconEventHandler for PlayerEventHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        info!("🎮 Connected as player");
        info!("   Connection ID: {}", client_info.connection_id);
        info!("   Role: {:?}", client_info.role);
        if let Some(username) = &client_info.username {
            info!("   Username: {}", username);
        }
    }
    
    fn on_disconnected(&mut self, reason: String) {
        warn!("❌ Disconnected: {}", reason);
    }
    
    fn on_message(&mut self, message: IncomingMessage) {
        match message.message_type.as_str() {
            "chat_message" => {
                if let Ok(chat_data) = serde_json::from_value::<serde_json::Value>(message.data.clone()) {
                    let sender = chat_data.get("sender").and_then(|s| s.as_str()).unwrap_or("Unknown");
                    let msg = chat_data.get("message").and_then(|m| m.as_str()).unwrap_or("");
                    info!("💬 {}: {}", sender, msg);
                }
            }
            "server_info" => {
                info!("ℹ️  Server information received");
                if let Ok(pretty) = serde_json::to_string_pretty(&message.data) {
                    println!("{}", pretty);
                }
            }
            "authenticated" => {
                info!("✅ Authentication successful");
            }
            "command_response" => {
                info!("⚡ Command response received");
                if let Some(success) = message.success {
                    if success {
                        info!("   ✅ Command executed successfully");
                    } else {
                        warn!("   ❌ Command failed: {:?}", message.error);
                    }
                }
            }
            _ => {
                info!("📥 Received: {} - {}", message.message_type, 
                    serde_json::to_string(&message.data).unwrap_or_default());
            }
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
    
    // Get token from command line or environment
    let token = env::args().nth(1)
        .or_else(|| env::var("BCON_PLAYER_TOKEN").ok())
        .expect("Please provide player token as argument or BCON_PLAYER_TOKEN environment variable");
    
    let server_url = env::var("BCON_SERVER_URL")
        .unwrap_or_else(|_| "ws://localhost:8081".to_string());
    
    info!("🎮 Starting player client");
    info!("   Server: {}", server_url);
    
    // Create player client configuration
    let auth = AuthConfig::player(token);
    let config = BconConfig::authenticated(server_url, auth);
    let mut client = BconClient::new(config);
    
    // Connect to server
    info!("🔌 Connecting to Bcon server...");
    client.connect().await?;
    
    // Request server info
    info!("ℹ️  Requesting server information...");
    if let Err(e) = client.request_server_info().await {
        error!("Failed to request server info: {}", e);
    }
    
    // Wait a moment then send a chat message
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    info!("💬 Sending welcome chat message...");
    if let Err(e) = client.send_chat(
        "Hello from Rust player client! 👋".to_string(), 
        None // Send to default server
    ).await {
        error!("Failed to send chat message: {}", e);
    }
    
    // Start event loop to receive messages
    let handler = PlayerEventHandler;
    info!("🎧 Starting event loop...");
    client.start_event_loop(handler).await?;
    
    Ok(())
}