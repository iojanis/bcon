# Bcon Server

A high-performance WebSocket communication server written in Rust for Minecraft adapters and clients. Bcon provides dual WebSocket servers with role-based authentication, rate limiting, and message routing - all without persistent storage requirements.

## Features

- **Dual WebSocket Servers**: Separate servers for adapters (port 8082) and clients (port 8081)
- **Role-Based Authentication**: JWT-based auth with Guest, Player, Admin, and System roles
- **Rate Limiting**: Configurable rate limits per role with automatic IP banning
- **Message Routing**: Intelligent routing between adapters and clients based on roles
- **In-Memory KV Store**: Non-persistent key-value storage for runtime data
- **Native & WASM**: Compiles to both native binary and WebAssembly
- **Zero Dependencies**: No external databases or persistent storage required

## Quick Start

### Native Binary

```bash
# Build and run
cargo build --release
./target/release/bcon

# Or with custom configuration
./target/release/bcon --config config.json

# Generate example config
./target/release/bcon --generate-config example-config.json
```

### WASM (Web/Node.js)

```bash
# Build WASM packages
./wasm-build.sh

# Use in web applications
import init, { WasmBconServer } from 'bcon-server-wasm';

# Use in Node.js/Deno
const { WasmBconServer } = require('bcon-server-node');
```

## Architecture

### Connection Flow

1. **Adapter Connection** (Port 8082):
   - Minecraft server plugins/mods connect with Bearer token
   - Server ID encoded in token for multi-server support
   - Strict rate limiting for unauthenticated attempts

2. **Client Connection** (Port 8081):
   - Web clients, apps, and system services connect
   - Optional authentication (guests allowed)
   - Role-based message filtering

3. **Message Routing**:
   ```
   Adapter → System Clients → Other Clients
   Client Commands → System Clients → Adapters
   ```

### Authentication

JWT tokens with separate secrets for adapters and clients:

```rust
// Adapter token (in server plugin)
{
  "server_id": "server1",
  "server_name": "My Server",
  "iss": "bcon-server",
  "exp": 1234567890
}

// Client token
{
  "user_id": "user123",
  "name": "PlayerName",
  "role": "player", // guest, player, admin, system
  "exp": 1234567890
}
```

### Message Types

**From Adapters:**
- `player_joined` - Player joins server
- `player_left` - Player leaves server
- `custom_command_executed` - Command executed in-game
- `chat_message` - Chat message from game

**From Clients:**
- `auth` - Authentication
- `send_chat` - Send chat to game
- `execute_command` - Execute command (admin/system only)
- `get_server_info` - Get server information

## Configuration

### Environment Variables

```bash
BCON_ADAPTER_PORT=8082
BCON_CLIENT_PORT=8081
BCON_ADAPTER_SECRET=your_secure_adapter_secret_here
BCON_CLIENT_SECRET=your_secure_client_secret_here
BCON_LOG_LEVEL=info
BCON_GUEST_RATE_LIMIT=30
BCON_PLAYER_RATE_LIMIT=120
BCON_ADMIN_RATE_LIMIT=300
BCON_SYSTEM_RATE_LIMIT=1000
```

### Configuration File

```json
{
  "adapter_port": 8082,
  "client_port": 8081,
  "adapter_secret": "your_secure_adapter_secret_here_at_least_32_chars_long",
  "client_secret": "your_secure_client_secret_here_at_least_32_chars_long",
  "rate_limits": {
    "guest_requests_per_minute": 30,
    "player_requests_per_minute": 120,
    "admin_requests_per_minute": 300,
    "system_requests_per_minute": 1000,
    "unauthenticated_adapter_attempts_per_minute": 5,
    "ban_threshold": 100,
    "ban_duration_hours": 24
  },
  "server_info": {
    "name": "My Minecraft Server",
    "description": "A cool server with web integration",
    "url": "play.myserver.com",
    "minecraft_version": "1.20.4"
  }
}
```

## API Usage

### WASM (JavaScript/TypeScript)

```javascript
import init, { WasmBconServer } from 'bcon-server-wasm';

async function startServer() {
    await init();
    
    const config = {
        adapter_port: 8082,
        client_port: 8081,
        adapter_secret: "secure_adapter_secret_32_chars_min",
        client_secret: "secure_client_secret_32_chars_min",
        // ... other config options
    };

    const server = new WasmBconServer(JSON.stringify(config));
    await server.start();
    
    // Get metrics
    const metrics = server.get_metrics();
    console.log('Active connections:', metrics);
    
    // Use KV store
    server.kv_set("player:user123", JSON.stringify({
        name: "PlayerName",
        level: 25,
        last_seen: Date.now()
    }));
    
    const playerData = server.kv_get("player:user123");
    console.log('Player data:', JSON.parse(playerData));
}
```

### Native Rust

```rust
use bcon_server::{BconServer, BconConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BconConfig::from_env()?;
    let server = BconServer::new(config)?;
    
    // Start server (blocks until shutdown)
    server.start().await?;
    
    Ok(())
}
```

## Rate Limiting

Configurable rate limits per role:
- **Guest**: 30 requests/minute
- **Player**: 120 requests/minute  
- **Admin**: 300 requests/minute
- **System**: 1000 requests/minute
- **Unauthenticated Adapters**: 5 attempts/minute

Automatic IP banning after threshold violations.

## Message Examples

### Player Join Event

```json
{
  "eventType": "player_joined",
  "data": {
    "playerId": "uuid-here",
    "playerName": "PlayerName",
    "x": 100.5,
    "y": 64.0,
    "z": -200.3,
    "dimension": "minecraft:overworld",
    "ip": "192.168.1.100",
    "version": "1.20.4"
  }
}
```

### Chat Message from Web

```json
{
  "eventType": "send_chat",
  "data": {
    "message": "Hello from web!",
    "server_id": "server1"
  }
}
```

### Command Execution

```json
{
  "eventType": "execute_command",
  "data": {
    "command": "give PlayerName diamond 64",
    "server_id": "server1"
  }
}
```

## Building

### Prerequisites

- Rust 1.70+
- For WASM: `wasm-pack`

### Native Build

```bash
cargo build --release
```

### WASM Build

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM packages
./wasm-build.sh
```

### Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy
```

## Deployment

### Docker

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/bcon /usr/local/bin/
EXPOSE 8081 8082
CMD ["bcon"]
```

### Systemd Service

```ini
[Unit]
Description=Bcon WebSocket Server
After=network.target

[Service]
Type=simple
User=bcon
WorkingDirectory=/opt/bcon
ExecStart=/usr/local/bin/bcon --config /etc/bcon/config.json
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Security

- JWT tokens with configurable expiration
- Rate limiting with automatic IP banning
- Role-based access control
- Origin validation for WebSocket connections
- No persistent storage (reduces attack surface)

## Monitoring

### Metrics

```rust
let metrics = server.get_metrics();
println!("Active adapters: {}", metrics.active_adapters);
println!("Active clients: {}", metrics.active_clients);
println!("Messages/sec: {}", metrics.messages_per_second);
println!("Connection errors: {}", metrics.connection_errors);
```

### Health Check

HTTP endpoint at `/health` returns server status:

```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "active_connections": 42,
  "version": "0.1.0"
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Format code: `cargo fmt`
6. Run clippy: `cargo clippy`
7. Submit pull request

## License

MIT License - see LICENSE file for details.