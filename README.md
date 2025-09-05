# Bcon: Complete Minecraft Server Integration Platform

A comprehensive, high-performance WebSocket communication system that bridges Minecraft servers with external applications. Bcon enables real-time event streaming, command execution, and player interaction through web apps, Discord bots, and other services across **Fabric**, **Paper**, and **Folia** server platforms.

## System Overview

Bcon consists of three main components that work together:

- **bcon_server**: WebSocket relay server that handles multiple Minecraft adapters and clients
- **bcon_client**: Cross-platform client library for connecting to Bcon servers  
- **bcon_adapter**: Multi-platform Minecraft mod/plugin that connects servers to the Bcon ecosystem

## Architecture

```
[Fabric/Paper/Folia + Bcon Adapter] ←--→ [Bcon Server] ←--→ [Web Apps/Clients]
        (Multi-Platform Adapter)            (Relay Hub)        (System/Player Clients)
```

### Platform Support
- **Fabric**: Full mod integration with Fabric API
- **Paper**: Native Bukkit/Paper plugin support  
- **Folia**: Thread-safe implementation for regionized servers

### Flow
1. **Minecraft adapters** (plugins/mods) connect to the server and send game events
2. **Bcon server** receives, validates, and routes messages between components  
3. **System clients** receive all game events and can send commands back to adapters
4. **Player clients** can interact with specific servers they have permission for

## Features

### ✅ **Implemented & Tested**

- **Multi-protocol Support**: WebSocket servers for both adapters (port 8082) and clients (port 8081)
- **JWT Authentication**: Secure token-based authentication for all connections
- **Role-based Permissions**: Guest, Player, Admin, and System roles with different capabilities
- **Rate Limiting**: Configurable rate limits per role to prevent abuse
- **Real-time Event Streaming**: Live Minecraft events (player join/leave, chat, server status)
- **Command Execution**: Send commands from clients back to Minecraft servers
- **Connection Management**: Automatic connection tracking, heartbeat monitoring
- **Message Routing**: Efficient routing of messages between adapters and clients
- **Cross-platform Client**: Native Rust client that works on Windows, macOS, and Linux
- **Configuration Management**: Flexible JSON-based configuration system

### 🚀 **Proven Working** 
- ✅ System client authentication and connection
- ✅ Player client authentication and connection  
- ✅ Minecraft adapter authentication and event streaming
- ✅ Real-time event processing (server lifecycle, world loading, etc.)
- ✅ Message routing from adapters to system clients
- ✅ Multiple simultaneous connections
- ✅ Token generation and validation

## Quick Start

### 1. Generate Authentication Tokens

```bash
# Generate system client token (receives all events, can send commands)
cd bcon_server
./target/release/token_gen --type client --role system --username SystemClient

# Generate player token (limited permissions)
./target/release/token_gen --type client --role player --username PlayerName

# Generate adapter token for Minecraft server
./target/release/token_gen --type adapter --server-id server_name --server-name "Display Name"
```

### 2. Start the Server

```bash
cd bcon_server
./target/release/bcon --config config.json
```

Server will start listening on:
- Port 8082: Adapter connections (Minecraft servers)
- Port 8081: Client connections (web apps, bots, etc.)

### 3. Connect Clients

```bash
cd bcon_client

# Connect as system client (receives all server events)
./target/release/bcon_client system --token "your_system_token"

# Connect as player
./target/release/bcon_client player --token "your_player_token"

# Connect as guest (no token required)
./target/release/bcon_client guest
```

### 4. Install Bcon Adapter

Choose the appropriate adapter for your Minecraft server platform:

#### Fabric Servers
Download the Bcon Fabric mod from [Modrinth](https://modrinth.com/mod/bcon) and install it in your `mods/` folder.

#### Paper/Spigot Servers  
Download the Bcon Paper plugin from [Modrinth](https://modrinth.com/plugin/bcon) and install it in your `plugins/` folder.

#### Folia Servers
Use the Paper plugin - it automatically detects and supports Folia's regionized threading.

### 5. Configure Minecraft Integration

Edit the `plugins/bcon/config.json` file in your Minecraft server:

```json
{
  "jwtToken": "your_adapter_token_here",
  "serverUrl": "ws://your-bcon-server:8082",
  "serverId": "your_server_id",
  "serverName": "Your Server Display Name"
}
```

## Configuration

The server uses `config.json` for configuration:

```json
{
  "adapter_port": 8082,
  "client_port": 8081,
  "adapter_secret": "your_32_char_minimum_adapter_secret_here",
  "client_secret": "your_32_char_minimum_client_secret_here",
  "rate_limits": {
    "guest_requests_per_minute": 30,
    "player_requests_per_minute": 120,
    "admin_requests_per_minute": 300,
    "system_requests_per_minute": 1000,
    "unauthenticated_adapter_attempts_per_minute": 5,
    "window_duration_seconds": 60,
    "ban_threshold": 100,
    "ban_duration_hours": 24
  },
  "heartbeat_interval_seconds": 30,
  "connection_timeout_seconds": 300,
  "log_level": "info"
}
```

## Client Types & Permissions

| Role | Description | Permissions |
|------|-------------|-------------|
| **Guest** | Unauthenticated users | Limited read access |
| **Player** | Authenticated players | Chat, player commands |
| **Admin** | Server administrators | All player permissions + admin commands |
| **System** | Applications & services | All permissions + receive all events |

## Event Types

The system handles various Minecraft events:

- **Player Events**: `player_joined`, `player_left`, `player_moved`
- **Chat Events**: `chat_message`, `whisper`
- **Server Events**: `server_started`, `server_stopped`, `server_starting`
- **World Events**: `world_load`, `world_save`
- **Custom Events**: Extensible for custom plugin events

## Message Format

### Client to Server (Authentication)
```json
{
  "eventType": "auth",
  "data": {
    "token": "your_jwt_token_here"
  }
}
```

### Server to Client (Event)
```json
{
  "type": "player_joined",
  "data": {
    "playerId": "uuid",
    "playerName": "PlayerName", 
    "x": 100.0,
    "y": 64.0,
    "z": 200.0,
    "dimension": "minecraft:overworld"
  },
  "timestamp": 1638360000
}
```

### Client to Adapter (Command via Server)
```json
{
  "eventType": "execute_command",
  "data": {
    "command": "say Hello from web!",
    "server_id": "minecraft_server_1"
  }
}
```

## Building from Source

### Requirements
- Rust 1.70+ 
- Git

### Build Server
```bash
cd bcon_server
cargo build --release
```

### Build Client
```bash
cd bcon_client  
cargo build --release
```

### Binaries Location
- Server: `bcon_server/target/release/bcon`
- Token Generator: `bcon_server/target/release/token_gen` 
- Client: `bcon_client/target/release/bcon_client`

## Development

### Running Tests
```bash
# Server tests
cd bcon_server && cargo test

# Client tests  
cd bcon_client && cargo test
```

### Development Mode
```bash
# Run with debug logging
RUST_LOG=debug ./target/release/bcon --config config.json
```

## Security Features

- **JWT Token Authentication**: All connections require valid JWT tokens
- **Rate Limiting**: Per-IP and per-role rate limiting to prevent abuse
- **Connection Monitoring**: Automatic heartbeat monitoring and timeout detection
- **IP Banning**: Automatic temporary bans for excessive rate limit violations
- **Input Validation**: All messages are validated before processing

## Use Cases

### 1. **Web Dashboard**
Connect a web app as a system client to display real-time server status, player locations, and chat.

### 2. **Discord Bot**
Build Discord bots that relay chat between Discord and Minecraft, show server status, execute commands.

### 3. **Mobile Apps** 
Create mobile apps that let players check server status, read chat, send messages while away.

### 4. **Analytics & Monitoring**
System clients can collect and analyze all server events for player behavior analytics, performance monitoring.

### 5. **Automation Systems**
Build automated systems that respond to events - restart servers, moderate chat, manage player groups.

## Troubleshooting

### Connection Issues
- Verify server is running and ports are accessible
- Check JWT token is valid and not expired
- Ensure correct server URL (ws:// not http://)

### Authentication Failures  
- Regenerate tokens if expired
- Verify adapter and client secrets match between token generation and server config
- Check token format and encoding

### Missing Events
- Ensure client has appropriate role permissions
- Verify Minecraft adapter is connected and sending events
- Check server logs for routing errors

## Technical Details

- **Language**: Rust
- **WebSocket Library**: tokio-tungstenite  
- **Authentication**: JWT with HMAC-SHA256
- **Serialization**: JSON with serde
- **Async Runtime**: Tokio
- **Logging**: tracing
- **Architecture**: Multi-threaded async with connection pooling

## License

MIT License - see LICENSE file for details.