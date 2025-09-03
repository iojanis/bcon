# Bcon Client

A WebSocket client library for connecting to Bcon servers with support for all client roles. Works natively, in Node.js/Deno, and in web browsers via WASM.

## Features

- **Multi-platform**: Native Rust, Node.js/Deno via WASM, and browser support
- **Role-based Authentication**: Support for Guest, Player, Admin, and System roles
- **Real-time Communication**: WebSocket-based bidirectional messaging
- **Type-safe**: Full TypeScript definitions when used in JavaScript environments
- **Comprehensive CLI**: Command-line interface for testing and automation
- **Event-driven Architecture**: Clean event handler pattern
- **Automatic Reconnection**: Configurable reconnection logic
- **Message Tracking**: Request/response correlation and statistics

## Installation

### Native Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
bcon_client = { path = "../bcon_client", features = ["native"] }
```

### WASM for Node.js/Deno/Browser

```bash
# Build WASM package
wasm-pack build --target web --out-dir pkg

# Use in Node.js
npm install ./pkg

# Use in Deno
import init, { WasmBconClient } from './pkg/bcon_client.js';
```

### CLI Tool

```bash
# Install from source
cargo install --path . --features native

# Or run directly
cargo run --bin bcon_client --features native -- --help
```

## Quick Start

### System Client (Rust)

```rust
use bcon_client::{
    BconClient, BconConfig, BconEventHandler,
    auth::AuthConfig,
    message::IncomingMessage,
    ClientInfo, BconError,
};

struct MyEventHandler;

impl BconEventHandler for MyEventHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        println!("Connected as {:?}", client_info.role);
    }
    
    fn on_message(&mut self, message: IncomingMessage) {
        if message.is_from_adapter() {
            // Handle adapter events (only system clients receive these)
            println!("Adapter event: {}", message.message_type);
        }
    }
    
    fn on_error(&mut self, error: BconError) {
        eprintln!("Error: {}", error);
    }
    
    fn on_disconnected(&mut self, reason: String) {
        println!("Disconnected: {}", reason);
    }
    
    fn on_auth_failed(&mut self, reason: String) {
        eprintln!("Auth failed: {}", reason);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BconConfig::system(
        "ws://localhost:8081".to_string(),
        "your_system_token_here".to_string()
    );
    
    let mut client = BconClient::new(config);
    client.connect().await?;
    
    // Send command to adapter
    client.send_adapter_command(
        Some("minecraft_server_1".to_string()),
        "get_player_list".to_string(),
        serde_json::json!({})
    )?;
    
    // Start event loop
    let handler = MyEventHandler;
    client.start_event_loop(handler).await?;
    
    Ok(())
}
```

### Player Client (Rust)

```rust
use bcon_client::{BconClient, BconConfig, auth::AuthConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = AuthConfig::player("username".to_string(), "password".to_string());
    let config = BconConfig::authenticated("ws://localhost:8081".to_string(), auth);
    
    let mut client = BconClient::new(config);
    client.connect().await?;
    
    // Send chat message
    client.send_chat("Hello from Rust client!".to_string(), None)?;
    
    // Request server info
    client.request_server_info()?;
    
    Ok(())
}
```

### JavaScript/TypeScript (Browser)

```html
<script type="module">
import init, { WasmBconClientBuilder } from './pkg/bcon_client.js';

await init();

const client = new WasmBconClientBuilder("ws://localhost:8081")
    .with_auth_token("your_token", "system")
    .with_timeout(30000)
    .build();

await client.connect();

// Send message
client.send_js_message(JSON.stringify({
    eventType: "custom_command",
    data: { action: "test" },
    messageId: "msg-123",
    timestamp: Date.now()
}));

// Set up event handlers
client.on_message((message) => {
    console.log("Received:", JSON.parse(message));
});

client.on_connection(() => {
    console.log("Connected to server");
});

client.on_error((error) => {
    console.error("Error:", error);
});
</script>
```

### CLI Examples

```bash
# Connect as guest
bcon_client guest --interactive --server ws://localhost:8081

# Connect as player
bcon_client player --username myuser --password mypass --interactive

# Connect as system client
bcon_client system --token "your_jwt_token" --interactive

# Send single message
bcon_client send --role system --token "token" --event-type "status_check" --data "{}"

# Connect as admin with custom server
bcon_client admin --token "admin_token" --server ws://production:8081 --interactive
```

## Client Roles

### Guest
- **Authentication**: None required
- **Permissions**: Read-only access to public information
- **Use Case**: Anonymous users, public dashboards

### Player
- **Authentication**: Username/password or API key
- **Permissions**: Send chat messages, view server info
- **Use Case**: Regular game players

### Admin
- **Authentication**: JWT token
- **Permissions**: Execute server commands, manage players
- **Use Case**: Server administrators

### System
- **Authentication**: JWT token
- **Permissions**: Full access, receive all events, send adapter commands
- **Use Case**: Web applications, bots, monitoring systems

## Message Flow

The Bcon server implements a pure relay pattern:

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Minecraft  │────│    Bcon     │────│   System    │
│   Server    │    │   Server    │    │   Client    │
│  (Adapter)  │    │   (Relay)   │    │  (Your App) │
└─────────────┘    └─────────────┘    └─────────────┘
                           │
                           │
                   ┌─────────────┐
                   │   Other     │
                   │  Clients    │
                   │ (Web/Game)  │
                   └─────────────┘
```

- **Adapters → System Clients**: All adapter events go to system clients only
- **System Clients → Adapters**: Commands and control messages
- **System Clients → Other Clients**: Filtered events based on business logic
- **Other Clients → System Clients**: Requests for processing

## Configuration

```rust
use bcon_client::{BconConfig, auth::AuthConfig};

let config = BconConfig {
    server_url: "ws://localhost:8081".to_string(),
    auth: Some(AuthConfig::system("token".to_string())),
    connect_timeout: 30000,      // 30 seconds
    heartbeat_interval: 30000,   // 30 seconds
    max_reconnect_attempts: 5,   // 0 = infinite
    reconnection_delay: 5000,    // 5 seconds
};
```

## Event Handling

```rust
impl BconEventHandler for MyHandler {
    fn on_connected(&mut self, client_info: ClientInfo) {
        // Connection established and authenticated
        println!("Connected: {:?}", client_info.role);
    }
    
    fn on_message(&mut self, message: IncomingMessage) {
        // Check if message came from an adapter
        if message.is_from_adapter() {
            if let Ok(relay_msg) = message.extract_relay_message() {
                println!("Adapter {}: {}", 
                    relay_msg.source_id.unwrap_or_default(),
                    relay_msg.message_type
                );
            }
        }
        
        // Handle specific message types
        match message.message_type.as_str() {
            "player_joined" => { /* handle player join */ }
            "chat_message" => { /* handle chat */ }
            "server_status" => { /* handle status update */ }
            _ => { /* handle other messages */ }
        }
    }
    
    fn on_error(&mut self, error: BconError) {
        match error {
            BconError::Connection(_) => { /* handle connection errors */ }
            BconError::Authentication(_) => { /* handle auth errors */ }
            BconError::PermissionDenied { role } => { /* handle permission errors */ }
            _ => { /* handle other errors */ }
        }
    }
    
    // ... other handlers
}
```

## Building WASM

```bash
# Install wasm-pack if you haven't already
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web
wasm-pack build --target web --features wasm

# Build for Node.js
wasm-pack build --target nodejs --features wasm

# Build for bundlers (webpack, etc.)
wasm-pack build --target bundler --features wasm
```

## Examples

- [`examples/system_client.rs`](examples/system_client.rs) - System client that receives adapter events
- [`examples/player_client.rs`](examples/player_client.rs) - Player client that sends chat messages  
- [`examples/web_example.html`](examples/web_example.html) - Web browser example with UI
- CLI tool with interactive mode for testing

## Development

```bash
# Run tests
cargo test

# Run native example
cargo run --example system_client --features native -- "your_token"

# Run CLI tool
cargo run --bin bcon_client --features native -- system --token "your_token" --interactive

# Build for all targets
cargo build --features native
wasm-pack build --target web --features wasm
```

## Error Handling

```rust
use bcon_client::BconError;

match client.send_chat("Hello".to_string(), None) {
    Ok(()) => println!("Message sent"),
    Err(BconError::NotConnected) => println!("Not connected to server"),
    Err(BconError::PermissionDenied { role }) => {
        println!("Permission denied for role: {:?}", role)
    }
    Err(e) => println!("Other error: {}", e),
}
```

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for your changes
4. Ensure all tests pass
5. Submit a pull request

For questions or support, please open an issue on GitHub.