use bcon_client::{
    BconClient, BconConfig, BconEventHandler, BconError,
    auth::{AuthConfig, ClientRole},
    message::{OutgoingMessage, IncomingMessage, MessageBuilder},
    ClientInfo, MessageStats,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json;
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, info, warn, Level};
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "bcon_client")]
#[command(about = "A CLI client for connecting to Bcon servers")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL to connect to
    #[arg(short, long, default_value = "ws://localhost:8081")]
    server: String,

    /// Log level
    #[arg(short, long, value_enum, default_value = "info")]
    log_level: LogLevel,

    /// Connection timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Heartbeat interval in seconds
    #[arg(long, default_value = "30")]
    heartbeat: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Connect as a guest (no authentication)
    Guest {
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// Connect as a player with token
    Player {
        /// JWT token
        #[arg(short, long)]
        token: String,
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// Connect as an admin with token
    Admin {
        /// JWT token
        #[arg(short, long)]
        token: String,
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// Connect as a system client
    System {
        /// JWT token
        #[arg(short, long)]
        token: String,
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },
    /// Send a single message and exit
    Send {
        /// Authentication token (for non-guest)
        #[arg(short, long)]
        token: Option<String>,
        /// Client role
        #[arg(short, long, value_enum, default_value = "guest")]
        role: CliRole,
        /// Event type
        #[arg(short, long)]
        event_type: String,
        /// JSON data payload
        #[arg(short, long)]
        data: String,
    },
}

#[derive(ValueEnum, Clone)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(ValueEnum, Clone)]
enum CliRole {
    Guest,
    Player,
    Admin,
    System,
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => Level::ERROR,
            LogLevel::Warn => Level::WARN,
            LogLevel::Info => Level::INFO,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl From<CliRole> for ClientRole {
    fn from(role: CliRole) -> Self {
        match role {
            CliRole::Guest => ClientRole::Guest,
            CliRole::Player => ClientRole::Player,
            CliRole::Admin => ClientRole::Admin,
            CliRole::System => ClientRole::System,
        }
    }
}

// Event handler for CLI
struct CliEventHandler {
    role: ClientRole,
    interactive: bool,
}

impl BconEventHandler for CliEventHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        println!("✅ Connected to server");
        println!("   Connection ID: {}", client_info.connection_id);
        println!("   Role: {:?}", client_info.role);
        if let Some(username) = client_info.username {
            println!("   Username: {}", username);
        }
        println!();

        if self.interactive {
            println!("🎮 Interactive mode - Type commands or 'help' for assistance");
            println!("   Type 'quit' to exit");
            println!();
        }
    }

    fn on_disconnected(&mut self, reason: String) {
        println!("❌ Disconnected: {}", reason);
    }

    fn on_message(&mut self, message: IncomingMessage) {
        println!("📥 Received: {}", message.message_type);

        // Pretty print the message
        if let Ok(pretty) = serde_json::to_string_pretty(&message) {
            println!("{}", pretty);
        } else {
            println!("{:?}", message);
        }
        println!();
    }

    fn on_error(&mut self, error: BconError) {
        eprintln!("❌ Error: {}", error);
    }

    fn on_auth_failed(&mut self, reason: String) {
        eprintln!("🔐 Authentication failed: {}", reason);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::from(cli.log_level))
        .init();

    // Create config
    let config = BconConfig {
        server_url: cli.server.clone(),
        connect_timeout: cli.timeout * 1000,
        heartbeat_interval: cli.heartbeat * 1000,
        auth: None, // Will be set based on command
        ..Default::default()
    };

    match cli.command {
        Commands::Guest { interactive } => {
            run_client(config, None, interactive).await?;
        }
        Commands::Player { token, interactive } => {
            let auth = AuthConfig::player(token);
            run_client(config, Some(auth), interactive).await?;
        }
        Commands::Admin { token, interactive } => {
            let auth = AuthConfig::admin(token);
            run_client(config, Some(auth), interactive).await?;
        }
        Commands::System { token, interactive } => {
            let auth = AuthConfig::system(token);
            run_client(config, Some(auth), interactive).await?;
        }
        Commands::Send { token, role, event_type, data } => {
            let auth = if let Some(token) = token {
                Some(AuthConfig::Token {
                    token,
                    role: role.into()
                })
            } else {
                None
            };

            send_single_message(config, auth, event_type, data).await?;
        }
    }

    Ok(())
}

async fn run_client(mut config: BconConfig, auth: Option<AuthConfig>, interactive: bool) -> Result<(), Box<dyn std::error::Error>> {
    config.auth = auth.clone();

    let role = auth.as_ref().map(|a| a.expected_role()).unwrap_or(ClientRole::Guest);
    let mut client = BconClient::new(config);

    println!("🔌 Connecting to Bcon server...");
    client.connect().await?;

    if interactive {
        // Run interactive mode
        run_interactive_mode(client, role).await?;
    } else {
        // Just listen for messages
        let handler = CliEventHandler { role, interactive };
        client.start_event_loop(handler).await?;
    }

    Ok(())
}

async fn run_interactive_mode(mut client: BconClient, role: ClientRole) -> Result<(), Box<dyn std::error::Error>> {
    // For simplicity in interactive mode, we'll use a channel-based approach
    // to avoid the complex deadlock situation with shared client state
    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<OutgoingMessage>();
    let client_clone = std::sync::Arc::new(tokio::sync::Mutex::new(client));
    let client_for_events = client_clone.clone();

    // Start event loop in background
    let handler = CliEventHandler { role: role.clone(), interactive: true };
    tokio::spawn(async move {
        let mut client_guard = client_for_events.lock().await;
        if let Err(e) = client_guard.start_event_loop(handler).await {
            eprintln!("Event loop error: {}", e);
        }
    });

    // Start message sender in background
    let client_for_sending = client_clone.clone();
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let mut client_guard = client_for_sending.lock().await;
            if let Err(e) = client_guard.send_message(message).await {
                eprintln!("Failed to send message: {}", e);
            }
            // Release lock immediately after sending
            drop(client_guard);
        }
    });

    // Handle user input
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    // Give the event loop a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    loop {
        print!("> ");
        io::stdout().flush()?;

        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                if input == "quit" || input == "exit" {
                    break;
                }

                if input == "help" {
                    show_help(&role);
                    continue;
                }

                if input == "stats" {
                    // Try to get stats safely
                    let client_guard = client_clone.lock().await;
                    let stats = client_guard.get_stats();
                    println!("📊 Statistics:");
                    println!("   Sent: {}", stats.sent);
                    println!("   Received: {}", stats.received);
                    println!("   Errors: {}", stats.errors);
                    println!("   Reconnections: {}", stats.reconnections);
                    drop(client_guard);
                    continue;
                }

                // Parse and send command via channel
                if let Some(message) = create_message_from_input(input, &role).await {
                    if let Err(_) = tx.send(message) {
                        eprintln!("❌ Failed to queue message");
                    } else {
                        println!("✅ Command queued for sending");
                    }
                }
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }

    println!("👋 Goodbye!");
    client_clone.lock().await.disconnect().await?;
    Ok(())
}

async fn create_message_from_input(input: &str, role: &ClientRole) -> Option<OutgoingMessage> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let command = parts[0];
    let args = parts.get(1).unwrap_or(&"").trim();

    match command {
        "heartbeat" | "ping" => {
            Some(OutgoingMessage::heartbeat())
        }
        "info" => {
            Some(OutgoingMessage::get_server_info())
        }
        "chat" if *role != ClientRole::Guest => {
            if args.is_empty() {
                println!("Usage: chat <message>");
                return None;
            }
            Some(OutgoingMessage::chat_message(args.to_string(), None)
                .with_timeout(30000).requires_acknowledgment())
        }
        "command" if matches!(role, ClientRole::Admin | ClientRole::System) => {
            if args.is_empty() {
                println!("Usage: command <command>");
                return None;
            }
            Some(OutgoingMessage::execute_command(args.to_string(), None)
                .with_timeout(30000).requires_acknowledgment())
        }
        "adapter" if *role == ClientRole::System => {
            let adapter_parts: Vec<&str> = args.splitn(2, ' ').collect();
            if adapter_parts.len() < 2 {
                println!("Usage: adapter <command_type> <json_data>");
                return None;
            }

            let command_type = adapter_parts[0].to_string();
            match serde_json::from_str::<serde_json::Value>(adapter_parts[1]) {
                Ok(data) => {
                    Some(OutgoingMessage::adapter_command(None, command_type, data))
                }
                Err(e) => {
                    println!("❌ Invalid JSON: {}", e);
                    None
                }
            }
        }
        "rcon" if matches!(role, ClientRole::Admin | ClientRole::System) => {
            if args.is_empty() {
                println!("Usage: rcon <command>");
                return None;
            }
            
            Some(OutgoingMessage::new(
                "rcon_command".to_string(),
                serde_json::json!({
                    "command": args,
                    "server_id": null
                })
            ).with_timeout(30000).requires_acknowledgment())
        }
        "send" => {
            let send_parts: Vec<&str> = args.splitn(2, ' ').collect();
            if send_parts.len() < 2 {
                println!("Usage: send <event_type> <json_data>");
                return None;
            }

            let event_type = send_parts[0].to_string();
            match serde_json::from_str::<serde_json::Value>(send_parts[1]) {
                Ok(data) => {
                    Some(OutgoingMessage::new(event_type, data)
                        .with_timeout(30000).requires_acknowledgment())
                }
                Err(e) => {
                    println!("❌ Invalid JSON: {}", e);
                    None
                }
            }
        }
        _ => {
            println!("❓ Unknown command: {}", command);
            println!("   Type 'help' for available commands");
            None
        }
    }
}


async fn handle_user_input(client: &mut BconClient, input: &str, role: &ClientRole) -> std::result::Result<(), BconError> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    let command = parts[0];
    let args = parts.get(1).unwrap_or(&"").trim();

    match command {
        "heartbeat" | "ping" => {
            client.send_heartbeat().await?;
            println!("💓 Heartbeat sent");
        }
        "info" => {
            client.request_server_info().await?;
            println!("ℹ️  Server info requested");
        }
        "chat" if *role != ClientRole::Guest => {
            if args.is_empty() {
                println!("Usage: chat <message>");
                return Ok(());
            }
            
            let chat_message = OutgoingMessage::chat_message(args.to_string(), None)
                .with_timeout(30000).requires_acknowledgment();
            client.send_message(chat_message).await?;
            println!("💬 Chat message sent (requires system client to process)");
        }
        "command" if matches!(role, ClientRole::Admin | ClientRole::System) => {
            if args.is_empty() {
                println!("Usage: command <command>");
                return Ok(());
            }
            
            let command_message = OutgoingMessage::execute_command(args.to_string(), None)
                .with_timeout(30000).requires_acknowledgment();
            client.send_message(command_message).await?;
            println!("⚡ Command sent (requires system client to process)");
        }
        "adapter" if *role == ClientRole::System => {
            let adapter_parts: Vec<&str> = args.splitn(2, ' ').collect();
            if adapter_parts.len() < 2 {
                println!("Usage: adapter <command_type> <json_data>");
                return Ok(());
            }

            let command_type = adapter_parts[0].to_string();
            let data: serde_json::Value = serde_json::from_str(adapter_parts[1])
                .map_err(|e| BconError::MessageParsing(e.to_string()))?;

            client.send_adapter_command(None, command_type, data).await?;
            println!("🔌 Adapter command sent");
        }
        "rcon" if matches!(role, ClientRole::Admin | ClientRole::System) => {
            if args.is_empty() {
                println!("Usage: rcon <command>");
                return Ok(());
            }
            
            let rcon_message = OutgoingMessage::new(
                "rcon_command".to_string(),
                serde_json::json!({
                    "command": args,
                    "server_id": null
                })
            ).with_timeout(30000).requires_acknowledgment();
            
            client.send_message(rcon_message).await?;
            println!("🔧 RCON command sent (waiting for response...)");
        }
        "send" => {
            let send_parts: Vec<&str> = args.splitn(2, ' ').collect();
            if send_parts.len() < 2 {
                println!("Usage: send <event_type> <json_data>");
                return Ok(());
            }

            let event_type = send_parts[0].to_string();
            let data: serde_json::Value = serde_json::from_str(send_parts[1])
                .map_err(|e| BconError::MessageParsing(e.to_string()))?;

            let message = OutgoingMessage::new(event_type, data)
                .with_timeout(30000).requires_acknowledgment();
            client.send_message(message).await?;
            println!("📤 Message sent (with acknowledgment tracking)");
        }
        _ => {
            println!("❓ Unknown command: {}", command);
            println!("   Type 'help' for available commands");
        }
    }

    Ok(())
}

fn show_help(role: &ClientRole) {
    println!("📖 Available commands:");
    println!("   help        - Show this help");
    println!("   quit/exit   - Exit the client");
    println!("   stats       - Show connection statistics");
    println!("   heartbeat   - Send heartbeat/ping");
    println!("   info        - Request server information");

    if *role != ClientRole::Guest {
        println!("   chat <msg>  - Send chat message");
    }

    if matches!(role, ClientRole::Admin | ClientRole::System) {
        println!("   command <cmd> - Execute server command");
        println!("   rcon <cmd>    - Execute RCON command");
    }

    if *role == ClientRole::System {
        println!("   adapter <type> <json> - Send command to adapter");
    }

    println!("   send <type> <json> - Send custom message");
    println!();
}

async fn send_single_message(mut config: BconConfig, auth: Option<AuthConfig>, event_type: String, data: String) -> Result<(), Box<dyn std::error::Error>> {
    config.auth = auth;

    // Parse JSON data
    let json_data: serde_json::Value = serde_json::from_str(&data)?;

    // Create and connect client
    let mut client = BconClient::new(config);
    client.connect().await?;

    // Send message
    let message = OutgoingMessage::new(event_type.clone(), json_data);
    client.send_message(message).await?;

    println!("✅ Message sent: {}", event_type);

    // Wait a moment for any response
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Disconnect
    client.disconnect().await?;

    Ok(())
}
