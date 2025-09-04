# Bcon Client Setup Guide

This guide covers how to use the Bcon Client to connect to Bcon servers, receive real-time Minecraft events, and send commands back to servers.

## Quick Start

### 1. Build the Client
```bash
git clone https://github.com/your-org/bcon.git
cd bcon/bcon_client

# Build release binary
cargo build --release
```

### 2. Get Authentication Token
Contact your server administrator to get a token, or generate one if you have access to the Bcon server:

```bash
# System client (full access)
cd ../bcon_server
./target/release/token_gen --type client --role system --username "MyApp"

# Player client (limited access)
./target/release/token_gen --type client --role player --username "PlayerName"
```

### 3. Connect to Server
```bash
cd ../bcon_client

# System client connection
./target/release/bcon_client system --token "your_system_token_here"

# Player client connection  
./target/release/bcon_client player --token "your_player_token_here"

# Guest connection (no token required)
./target/release/bcon_client guest
```

## Client Types

### System Client
Full access to all server events and commands:
```bash
./target/release/bcon_client system \
  --token "system_token" \
  --server "ws://localhost:8081" \
  --format json
```

**Capabilities:**
- ✅ Receive all server events
- ✅ Send commands to any connected server
- ✅ Manage dynamic commands
- ✅ Access server statistics

### Player Client
Limited access for individual players:
```bash
./target/release/bcon_client player \
  --token "player_token" \
  --server "ws://localhost:8081"
```

**Capabilities:**
- ✅ Receive public events (join/leave, chat)
- ✅ Send chat messages
- ✅ Execute allowed player commands
- ❌ Access sensitive server information

### Guest Client
Public access without authentication:
```bash
./target/release/bcon_client guest \
  --server "ws://localhost:8081"
```

**Capabilities:**
- ✅ Receive basic server status
- ✅ View public announcements
- ❌ Send commands
- ❌ Access player data

## Command Line Options

### Global Options
```bash
./target/release/bcon_client [OPTIONS] <CLIENT_TYPE>

OPTIONS:
  -s, --server <URL>     Server WebSocket URL [default: ws://localhost:8081]
  -t, --token <TOKEN>    Authentication token (required for system/player)
  -f, --format <FORMAT>  Output format: json, pretty, compact [default: pretty]
  -v, --verbose         Enable verbose logging
  -q, --quiet           Suppress non-essential output
  --timeout <SECONDS>   Connection timeout [default: 30]
  --reconnect           Auto-reconnect on connection loss
  --log-file <PATH>     Log output to file
```

### Output Formats

**Pretty Format** (default):
```
[2024-01-15 14:30:25] PLAYER_JOINED
  Player: Steve (uuid-1234)
  Location: 100.0, 64.0, 200.0 (minecraft:overworld)
  Health: 20.0/20.0, Level: 10
```

**JSON Format**:
```json
{
  "eventType": "player_joined",
  "timestamp": 1705323025,
  "data": {
    "playerId": "uuid-1234",
    "playerName": "Steve",
    "x": 100.0, "y": 64.0, "z": 200.0,
    "dimension": "minecraft:overworld",
    "health": 20.0, "maxHealth": 20.0, "level": 10
  }
}
```

**Compact Format**:
```
14:30:25 Steve joined at 100,64,200 (overworld)
14:30:30 <Steve> Hello everyone!
14:30:45 Alex left the game
```

## Interactive Mode

### System Client Interactive Commands
When running as system client, use these commands:

```bash
# Send server command
/cmd server_id say Hello from Bcon!

# Register dynamic command
/register teleport "Teleport command" --arg player --arg location

# List connected servers
/servers

# Get server info
/info server_id

# Send chat message
/chat server_id Hello everyone!

# Disconnect
/quit
```

### Player Client Interactive Commands
```bash
# Send chat message
/chat Hello everyone!

# List available commands
/commands

# Get help
/help

# Disconnect  
/quit
```

## Programming with the Client

### Rust Library Usage
```rust
use bcon_client::{BconClient, ClientConfig, ClientRole};

#[tokio::main]
async fn main() {
    let config = ClientConfig {
        server_url: "ws://localhost:8081".to_string(),
        token: Some("your_token".to_string()),
        role: ClientRole::System,
        auto_reconnect: true,
        ..Default::default()
    };
    
    let mut client = BconClient::new(config).await.unwrap();
    
    // Listen for events
    while let Some(event) = client.next_event().await {
        match event.event_type.as_str() {
            "player_joined" => {
                println!("Player joined: {}", event.data["playerName"]);
            },
            "chat_message" => {
                let player = &event.data["playerName"];
                let message = &event.data["message"];
                println!("<{}> {}", player, message);
            },
            _ => println!("Received: {}", event.event_type)
        }
    }
}
```

### WebSocket Integration
For web applications:

```javascript
const ws = new WebSocket('ws://localhost:8081');

// Authenticate
ws.onopen = () => {
    ws.send(JSON.stringify({
        eventType: 'auth',
        data: { token: 'your_token' }
    }));
};

// Handle events
ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    
    switch(data.eventType) {
        case 'player_joined':
            console.log(`${data.data.playerName} joined the server`);
            break;
        case 'chat_message':
            console.log(`<${data.data.playerName}> ${data.data.message}`);
            break;
    }
};

// Send command
function sendCommand(serverId, command) {
    ws.send(JSON.stringify({
        eventType: 'execute_command',
        data: {
            server_id: serverId,
            command: command
        }
    }));
}
```

## Event Types Reference

### Server Events
- `server_starting`, `server_started`, `server_stopping`, `server_stopped`
- `server_before_save`, `server_after_save` 
- `data_pack_reload_start`, `data_pack_reload_end`

### Player Events
- `player_joined`, `player_left`, `player_connection_init`
- `player_respawned`, `player_death`
- `player_break_block_before`, `player_break_block_after`
- `player_chat`

### World Events
- `world_load`, `world_unload`

### Entity Events  
- `entity_death`, `projectile_kill`

### Interaction Events
- `merchant_interaction`, `redstone_update`, `advancement_complete`

### Custom Events
- `custom_command_executed`

See [Event Documentation](../adapter_events.md) for complete details.

## Configuration File

Create `~/.bcon/config.toml` for persistent settings:

```toml
[client]
default_server = "ws://localhost:8081"
auto_reconnect = true
timeout_seconds = 30

[system]
token = "your_system_token"
format = "pretty"
log_level = "info"

[player]  
token = "your_player_token"
username = "YourUsername"

[display]
show_timestamps = true
color_output = true
compact_mode = false
```

## Monitoring and Logging

### Enable Detailed Logging
```bash
RUST_LOG=debug ./target/release/bcon_client system \
  --token "token" \
  --log-file client.log \
  --verbose
```

### Log Analysis
```bash
# Follow live logs
tail -f client.log

# Filter events
grep "player_joined" client.log
grep "ERROR" client.log

# Connection monitoring
grep -E "(Connected|Disconnected|Reconnecting)" client.log
```

### Health Monitoring
```bash
# Check connection status
./target/release/bcon_client system \
  --token "token" \
  --health-check

# Test connection
./target/release/bcon_client guest \
  --server "ws://localhost:8081" \
  --timeout 5 \
  --ping-only
```

## Scripting and Automation

### Event Processing Script
```bash
#!/bin/bash
# monitor-players.sh

./target/release/bcon_client system \
  --token "$SYSTEM_TOKEN" \
  --format json \
  --quiet \
| jq -r 'select(.eventType == "player_joined") | .data.playerName' \
| while read player; do
    echo "$(date): $player joined the server"
    # Send welcome message
    curl -X POST http://discord-webhook-url \
      -H "Content-Type: application/json" \
      -d "{\"content\": \"Welcome $player to the server!\"}"
done
```

### Chat Bot Example
```bash
#!/bin/bash
# simple-bot.sh

echo "Starting chat bot..."

./target/release/bcon_client system \
  --token "$SYSTEM_TOKEN" \
  --format json \
| jq -r 'select(.eventType == "player_chat") | "\(.data.playerName): \(.data.message)"' \
| while read line; do
    player=$(echo "$line" | cut -d: -f1)
    message=$(echo "$line" | cut -d: -f2-)
    
    # Respond to !help
    if [[ "$message" == *"!help"* ]]; then
        echo "Sending help message to $player"
        # Implementation depends on your needs
    fi
done
```

### Backup Trigger
```bash
#!/bin/bash
# backup-on-save.sh

./target/release/bcon_client system \
  --token "$SYSTEM_TOKEN" \
  --format json \
  --quiet \
| jq -r 'select(.eventType == "server_after_save")' \
| while read event; do
    echo "$(date): Server saved, triggering backup"
    rsync -av /path/to/minecraft/world/ /backups/$(date +%Y%m%d_%H%M%S)/
done
```

## Troubleshooting

### Connection Issues

**❌ "Connection refused"**
```bash
# Check server is running
curl -I http://server:8081/health

# Test WebSocket connection
websocat ws://server:8081
```

**❌ "Invalid token"**
```bash
# Verify token with server
cd ../bcon_server
./target/release/token_gen --verify --token "your_token"

# Generate new token if expired
./target/release/token_gen --type client --role system --username "NewClient"
```

**❌ "Timeout connecting"**
```bash
# Increase timeout
./target/release/bcon_client system \
  --token "token" \
  --timeout 60

# Check network connectivity
ping server-hostname
telnet server-hostname 8081
```

### Performance Issues

**🐌 High latency**
```bash
# Use compact format to reduce processing
./target/release/bcon_client system \
  --token "token" \
  --format compact

# Connect to local server
./target/release/bcon_client system \
  --server "ws://127.0.0.1:8081"
```

**💾 High memory usage**
```bash
# Disable event buffering
BCON_BUFFER_EVENTS=false ./target/release/bcon_client system

# Use streaming mode
./target/release/bcon_client system \
  --token "token" \
  --stream-only
```

## Building from Source

### Development Build
```bash
# Debug build with all features
cargo build --all-features

# Release build optimized
cargo build --release --all-features

# Cross-compile for different targets
cargo build --target x86_64-pc-windows-gnu
cargo build --target aarch64-apple-darwin
```

### Custom Features
```bash
# Build with TLS support
cargo build --features tls

# Build with metrics collection
cargo build --features metrics

# Minimal build
cargo build --no-default-features
```

## Support

- **Documentation**: [Full API Reference](https://bcon-docs.example.com/client)
- **Examples**: [GitHub Examples](https://github.com/your-org/bcon/tree/main/examples)
- **Issues**: [Report Problems](https://github.com/your-org/bcon/issues)
- **Discord**: [Community Support](https://discord.gg/bcon)