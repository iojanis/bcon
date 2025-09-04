# Bcon Complete Setup Guide

This comprehensive guide will walk you through setting up the complete Bcon ecosystem, from server installation to Minecraft integration and client connections.

## Table of Contents

1. [Quick Start with Docker](#quick-start-with-docker)
2. [Manual Installation](#manual-installation)
3. [Minecraft Server Integration](#minecraft-server-integration)
4. [Client Connections](#client-connections)
5. [Development Setup](#development-setup)
6. [Troubleshooting](#troubleshooting)

## Quick Start with Docker

The fastest way to get Bcon running is with Docker Compose, which sets up the entire stack automatically.

### Prerequisites
- Docker and Docker Compose installed
- At least 4GB available RAM

### 1. Clone and Start
```bash
git clone https://github.com/your-org/bcon.git
cd bcon
docker-compose up -d
```

This starts:
- **Bcon Server** on ports 8081 (clients) and 8082 (adapters)
- **Minecraft Server** (Paper) with Bcon adapter pre-configured
- **Example Client** that demonstrates real-time event streaming

### 2. Connect to Your Server
The Minecraft server will be available at `localhost:25565`. The default configuration includes:
- Server automatically connects to Bcon server
- Real-time event streaming enabled
- Dynamic command system active

### 3. View Live Events
```bash
# Connect as system client to see all events
docker-compose exec bcon-client bcon_client system --token "system_token"

# Or check the logs
docker-compose logs -f bcon-server
```

### 4. Minecraft Server Access
- **Server Address**: `localhost:25565`
- **Version**: 1.20.4 Paper
- **Pre-installed**: Bcon adapter plugin
- **Admin Password**: Check `docker-compose.yml` for the adapter token

## Manual Installation

For production environments or custom setups, follow these manual installation steps.

### Step 1: Install Bcon Server

```bash
# Download or build the server
cd bcon_server
cargo build --release

# Create configuration
cp config.example.json config.json
nano config.json  # Edit your configuration
```

### Step 2: Generate Tokens

```bash
# System client (for applications, bots, dashboards)
./target/release/token_gen --type client --role system --username "SystemClient"

# Player client (for individual players)  
./target/release/token_gen --type client --role player --username "PlayerName"

# Adapter token (for Minecraft servers)
./target/release/token_gen --type adapter --server-id "my_server" --server-name "My Minecraft Server"
```

**Important**: Save these tokens securely. You'll need them for client connections and Minecraft server configuration.

### Step 3: Start Bcon Server

```bash
./target/release/bcon --config config.json
```

The server will start and listen on:
- **Port 8081**: Client connections (web apps, bots, etc.)
- **Port 8082**: Adapter connections (Minecraft servers)

### Step 4: Install Bcon Client

```bash
cd ../bcon_client
cargo build --release

# Test system client connection
./target/release/bcon_client system --token "your_system_token_here"
```

## Minecraft Server Integration

### Fabric Servers

1. **Download the Mod**
   ```bash
   # Download from Modrinth or build from source
   cd bcon_adapter
   ./gradlew fabric:build
   ```

2. **Install**
   - Copy the `.jar` file to your server's `mods/` folder
   - Ensure Fabric API is also installed

3. **Configure**
   ```json
   # config/bcon/config.json
   {
     "jwtToken": "your_adapter_token_here",
     "serverUrl": "ws://your-bcon-server:8082",
     "serverId": "fabric_server_1",
     "serverName": "My Fabric Server",
     "enableBlueMap": true
   }
   ```

### Paper/Spigot Servers

1. **Download the Plugin**
   ```bash
   cd bcon_adapter  
   ./gradlew paper:build
   ```

2. **Install**
   - Copy the `.jar` file to your server's `plugins/` folder
   - Restart the server

3. **Configure**
   ```json
   # plugins/bcon/config.json
   {
     "jwtToken": "your_adapter_token_here", 
     "serverUrl": "ws://your-bcon-server:8082",
     "serverId": "paper_server_1",
     "serverName": "My Paper Server",
     "strictMode": false
   }
   ```

### Folia Servers

Folia servers use the same Paper plugin, which automatically detects and supports regionized threading:

1. **Use Paper Plugin**: The Paper build works automatically on Folia
2. **Configuration**: Same as Paper (above)
3. **Threading**: Plugin automatically uses Folia's region schedulers

## Client Connections

### System Clients (Applications, Bots, Dashboards)

System clients receive all server events and can execute commands:

```bash
# Command line client
./target/release/bcon_client system --token "system_token"

# Or programmatically
curl -H "Authorization: Bearer system_token" \
     -H "Upgrade: websocket" \
     ws://localhost:8081/
```

### Player Clients (Individual Players)

Player clients have limited permissions and can only interact with servers they have access to:

```bash
./target/release/bcon_client player --token "player_token"
```

### Guest Clients (Public Access)

Guest clients require no authentication but have very limited permissions:

```bash
./target/release/bcon_client guest
```

## Development Setup

### Building from Source

```bash
# Clone repository
git clone https://github.com/your-org/bcon.git
cd bcon

# Build server
cd bcon_server && cargo build --release && cd ..

# Build client  
cd bcon_client && cargo build --release && cd ..

# Build adapters (requires Java 21+)
cd bcon_adapter && ./gradlew build && cd ..
```

### Development Environment

```bash
# Start server in development mode
cd bcon_server
RUST_LOG=debug ./target/release/bcon --config config.json

# Connect development client
cd ../bcon_client
RUST_LOG=debug ./target/release/bcon_client system --token "dev_token"
```

### Testing

```bash
# Run server tests
cd bcon_server && cargo test

# Run client tests
cd bcon_client && cargo test

# Test adapter functionality
cd bcon_adapter && ./gradlew test
```

## Event Types Reference

The Bcon adapter sends comprehensive events from your Minecraft server:

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
- `custom_command_executed` (dynamic commands)

## Advanced Configuration

### Rate Limiting
```json
{
  "rate_limits": {
    "guest_requests_per_minute": 30,
    "player_requests_per_minute": 120, 
    "admin_requests_per_minute": 300,
    "system_requests_per_minute": 1000
  }
}
```

### Security Settings
```json
{
  "adapter_secret": "your_32_character_minimum_secret",
  "client_secret": "your_32_character_minimum_secret",
  "connection_timeout_seconds": 300,
  "heartbeat_interval_seconds": 30
}
```

### BlueMap Integration
```json
{
  "enableBlueMap": true,
  "bluemap_markers": {
    "enabled": true,
    "default_icon": "assets/poi.svg"
  }
}
```

## Troubleshooting

### Connection Issues

**Problem**: Adapter won't connect to server
```bash
# Check server is running
netstat -an | grep 8082

# Verify token
./target/release/token_gen --verify --token "your_token"

# Check server logs
tail -f server.log
```

**Problem**: Client authentication fails
```bash
# Regenerate token
./target/release/token_gen --type client --role system --username "NewClient"

# Test connection
./target/release/bcon_client system --token "new_token" --verbose
```

### Minecraft Integration Issues

**Problem**: Adapter plugin not loading
- Verify Java version (Java 21+ required)
- Check plugin dependencies (Fabric API for Fabric servers)
- Review server startup logs

**Problem**: Events not appearing
- Verify adapter token in Minecraft server config
- Check network connectivity to Bcon server
- Enable debug logging in adapter config

### Performance Issues

**Problem**: High memory usage
- Reduce event buffer sizes in config
- Lower heartbeat frequency
- Implement rate limiting

**Problem**: Connection drops
- Increase connection timeout
- Check network stability
- Review firewall settings

## Production Deployment

### Server Requirements
- **CPU**: 2+ cores recommended
- **RAM**: 4GB minimum, 8GB recommended
- **Network**: Stable connection, ports 8081-8082 open
- **OS**: Linux, macOS, or Windows

### Security Checklist
- [ ] Use strong JWT secrets (32+ characters)
- [ ] Enable rate limiting
- [ ] Configure appropriate role permissions
- [ ] Use HTTPS/WSS in production
- [ ] Regular token rotation
- [ ] Monitor connection logs

## Support and Community

- **Documentation**: [Full API docs](https://bcon-docs.example.com)
- **Issues**: [GitHub Issues](https://github.com/your-org/bcon/issues)
- **Discord**: [Community Server](https://discord.gg/bcon)
- **Wiki**: [Community Wiki](https://wiki.bcon.example.com)

## License

MIT License - see [LICENSE](LICENSE) file for details.
