# Bcon Adapter Getting Started Guide

This guide will walk you through installing, configuring, and using the Bcon Adapter on your Minecraft server. The adapter supports Fabric, Paper, and Folia platforms with a unified configuration system.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [First Connection](#first-connection)
5. [Dynamic Commands](#dynamic-commands)
6. [BlueMap Integration](#bluemap-integration)
7. [Development Setup](#development-setup)
8. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements
- **Minecraft Server**: Version 1.20.4+ recommended
- **Java**: Version 21 or higher
- **Bcon Server**: Must be running and accessible

### Platform Dependencies

#### Fabric
- [Fabric Loader](https://fabricmc.net/use/installer/) installed
- [Fabric API](https://modrinth.com/mod/fabric-api) mod

#### Paper/Spigot
- Paper/Spigot server version 1.20.4+
- No additional dependencies required

#### Folia
- Folia server version 1.20.4+
- Uses the same plugin as Paper

## Installation

### Step 1: Download the Adapter

#### Option A: Download from Modrinth
```bash
# Fabric mod
https://modrinth.com/mod/bcon

# Paper/Spigot plugin  
https://modrinth.com/plugin/bcon
```

#### Option B: Build from Source
```bash
git clone https://github.com/your-org/bcon.git
cd bcon/bcon_adapter

# Build Fabric mod
./gradlew fabric:build

# Build Paper plugin
./gradlew paper:build

# Built files will be in:
# fabric/build/libs/bcon-adapter-fabric-*.jar
# paper/build/libs/bcon-adapter-paper-*.jar
```

### Step 2: Install on Your Server

#### Fabric Installation
1. Copy `bcon-adapter-fabric-*.jar` to your server's `mods/` folder
2. Ensure `fabric-api-*.jar` is also in the `mods/` folder
3. Restart your server

#### Paper/Spigot Installation
1. Copy `bcon-adapter-paper-*.jar` to your server's `plugins/` folder
2. Restart your server

#### Folia Installation
1. Use the same Paper plugin jar file
2. Copy to `plugins/` folder and restart
3. The plugin automatically detects Folia and uses appropriate threading

### Step 3: Verify Installation

After restarting your server, check the logs:

**Fabric:**
```
[INFO] Loading mod 'bcon' version 1.0.0
[INFO] Bcon Adapter: Initializing for Fabric platform
```

**Paper/Folia:**
```
[INFO] Loading bcon v1.0.0
[INFO] Bcon Adapter: Initializing for Paper platform
[INFO] Bcon Adapter: Folia support detected (if running Folia)
```

## Configuration

### Step 1: Generate Adapter Token

First, you need an adapter token from your Bcon server:

```bash
cd bcon_server
./target/release/token_gen \
  --type adapter \
  --server-id "my_minecraft_server" \
  --server-name "My Minecraft Server" \
  --expires-in-days 365
```

Save the generated token - you'll need it for configuration.

### Step 2: Create Configuration

The adapter will create a default configuration file on first run:

**Fabric:** `config/bcon/config.json`  
**Paper/Folia:** `plugins/bcon/config.json`

### Step 3: Edit Configuration

```json
{
  "jwtToken": "YOUR_ADAPTER_TOKEN_HERE",
  "serverUrl": "ws://localhost:8082",
  "serverId": "my_minecraft_server",
  "serverName": "My Minecraft Server",
  "enableBlueMap": false,
  "strictMode": false,
  "reconnectInterval": 5000,
  "maxReconnectAttempts": 10,
  "eventBuffer": {
    "enabled": true,
    "maxSize": 1000
  },
  "commands": {
    "enableDynamic": true,
    "allowSubcommands": true,
    "requirePermissions": true
  }
}
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `jwtToken` | Adapter authentication token | `""` |
| `serverUrl` | Bcon server WebSocket URL | `"ws://localhost:8082"` |
| `serverId` | Unique server identifier | `"minecraft_server_1"` |
| `serverName` | Display name for your server | `"Minecraft Server"` |
| `enableBlueMap` | Enable BlueMap integration | `false` |
| `strictMode` | Strict event validation | `false` |
| `reconnectInterval` | Reconnection delay (ms) | `5000` |
| `maxReconnectAttempts` | Max reconnection attempts | `10` |
| `eventBuffer.enabled` | Buffer events during disconnection | `true` |
| `eventBuffer.maxSize` | Maximum buffered events | `1000` |
| `commands.enableDynamic` | Allow dynamic command registration | `true` |
| `commands.allowSubcommands` | Support subcommands | `true` |
| `commands.requirePermissions` | Enforce command permissions | `true` |

## First Connection

### Step 1: Start Your Bcon Server

```bash
cd bcon_server
./target/release/bcon --config config.json
```

### Step 2: Restart Your Minecraft Server

After configuring the adapter, restart your Minecraft server. You should see:

```
[INFO] Bcon Adapter: Connecting to ws://localhost:8082
[INFO] Bcon Adapter: Successfully connected to Bcon server
[INFO] Bcon Adapter: Authenticated as server 'my_minecraft_server'
[INFO] Bcon Adapter: Event streaming active
```

### Step 3: Test the Connection

Connect a client to verify events are flowing:

```bash
cd bcon_client
./target/release/bcon_client system --token "your_system_token"
```

Join your Minecraft server - you should see a `player_joined` event in the client.

## Dynamic Commands

### Registering Commands

Dynamic commands are registered via WebSocket messages to the Bcon server:

```json
{
  "eventType": "register_command",
  "data": {
    "server_id": "my_minecraft_server",
    "command": {
      "name": "heal",
      "description": "Heal a player to full health",
      "permission": "bcon.heal",
      "arguments": [
        {
          "name": "player",
          "type": "PLAYER",
          "required": true,
          "description": "Player to heal"
        },
        {
          "name": "amount",
          "type": "DOUBLE",
          "required": false,
          "description": "Amount to heal",
          "min": 1.0,
          "max": 20.0,
          "default": 20.0
        }
      ]
    }
  }
}
```

### Command with Subcommands

```json
{
  "name": "teleport",
  "description": "Teleportation commands",
  "permission": "bcon.teleport",
  "subcommands": [
    {
      "name": "to",
      "description": "Teleport to a player",
      "arguments": [
        {
          "name": "target",
          "type": "PLAYER",
          "required": true
        }
      ]
    },
    {
      "name": "coords",
      "description": "Teleport to coordinates",
      "arguments": [
        {
          "name": "x",
          "type": "DOUBLE",
          "required": true
        },
        {
          "name": "y",
          "type": "DOUBLE",
          "required": true
        },
        {
          "name": "z",
          "type": "DOUBLE",
          "required": true
        },
        {
          "name": "world",
          "type": "WORLD",
          "required": false
        }
      ]
    }
  ]
}
```

### Command Execution

When players execute dynamic commands, the adapter sends a `custom_command_executed` event:

```json
{
  "eventType": "custom_command_executed",
  "data": {
    "commandName": "heal",
    "executor": "PlayerName",
    "arguments": {
      "player": "TargetPlayer",
      "amount": 20.0
    },
    "result": "success",
    "message": "TargetPlayer has been healed to full health"
  }
}
```

### Argument Types

| Type | Description | Additional Properties |
|------|-------------|----------------------|
| `STRING` | Text input | `regex`, `minLength`, `maxLength` |
| `INTEGER` | Whole numbers | `min`, `max` |
| `DOUBLE` | Decimal numbers | `min`, `max`, `precision` |
| `BOOLEAN` | True/false values | - |
| `PLAYER` | Online player selection | `allowOffline` |
| `WORLD` | World/dimension selection | `allowNether`, `allowEnd` |
| `ITEM` | Item/block type | `allowAir`, `allowDamaged` |
| `LOCATION` | Coordinate input | `allowRelative` |
| `ENUM` | Predefined values | `values` |

## BlueMap Integration

### Enabling BlueMap Support

1. Install [BlueMap](https://bluemap.bluecolorconsole.org/) on your server
2. Set `"enableBlueMap": true` in your bcon config
3. Restart your server

### Adding Markers via WebSocket

```json
{
  "eventType": "add_marker",
  "data": {
    "server_id": "my_minecraft_server",
    "marker": {
      "id": "spawn_point",
      "label": "Server Spawn",
      "x": 0.0,
      "y": 64.0,
      "z": 0.0,
      "world": "world",
      "icon": "assets/poi.svg",
      "anchor": {
        "x": 25,
        "y": 45
      },
      "minDistance": 10,
      "maxDistance": 1000
    }
  }
}
```

### Marker Management Commands

#### Add/Update Marker
```json
{
  "eventType": "add_marker",
  "data": { /* marker definition */ }
}
```

#### Remove Marker
```json
{
  "eventType": "remove_marker", 
  "data": {
    "server_id": "my_minecraft_server",
    "marker_id": "spawn_point"
  }
}
```

#### List Markers
```json
{
  "eventType": "list_markers",
  "data": {
    "server_id": "my_minecraft_server"
  }
}
```

## Development Setup

### Building for Development

```bash
# Clone repository
git clone https://github.com/your-org/bcon.git
cd bcon/bcon_adapter

# Set up development environment
./gradlew setupDev

# Build and test
./gradlew build test
```

### Running Development Server

#### Fabric Development
```bash
# Run Minecraft client
./gradlew fabric:runClient

# Run dedicated server
./gradlew fabric:runServer
```

#### Paper Development
```bash
# Build plugin
./gradlew paper:build

# Copy to test server
cp paper/build/libs/*.jar ~/test-server/plugins/

# Start test server
cd ~/test-server && java -jar paper-*.jar
```

### Debug Mode

Enable debug logging by adding to your server's JVM arguments:
```bash
-Dbcon.debug=true -Dbcon.log.level=DEBUG
```

Or set environment variable:
```bash
export BCON_LOG_LEVEL=DEBUG
```

Debug logs will show detailed WebSocket communication, event processing, and connection status.

### Hot Reload Configuration

The adapter watches for configuration changes and reloads automatically:

```bash
# Edit config while server is running
nano config/bcon/config.json  # Changes applied immediately
```

## Troubleshooting

### Connection Issues

#### "Failed to connect to Bcon server"

**Check server status:**
```bash
curl -I http://localhost:8082/health
```

**Verify configuration:**
- Confirm `serverUrl` points to the correct Bcon server
- Check if port 8082 is accessible from your Minecraft server
- Verify no firewall is blocking the connection

**Test WebSocket connection:**
```bash
websocat ws://localhost:8082
```

#### "Authentication failed"

**Verify token:**
```bash
cd bcon_server
./target/release/token_gen --verify --token "your_adapter_token"
```

**Regenerate token if needed:**
```bash
./target/release/token_gen --type adapter --server-id "my_minecraft_server"
```

#### "Connection keeps dropping"

**Check logs for:**
- Network timeout errors
- Rate limiting messages  
- JWT expiration warnings

**Increase connection stability:**
```json
{
  "reconnectInterval": 10000,
  "maxReconnectAttempts": 20,
  "eventBuffer": {
    "enabled": true,
    "maxSize": 2000
  }
}
```

### Event Issues

#### "Events not appearing in clients"

**Check event registration:**
- Verify adapter is connected to Bcon server
- Confirm clients are connected and authenticated
- Check client role permissions

**Enable event debugging:**
```json
{
  "strictMode": true
}
```

**Check event buffer:**
- Events may be buffered during disconnections
- Increase `maxSize` if events are being dropped

#### "Custom commands not working"

**Verify command registration:**
- Commands must be registered via WebSocket before use
- Check `enableDynamic` is true in config
- Verify command syntax and permissions

**Common issues:**
- Missing required arguments
- Invalid argument types
- Permission denied for player
- Command name conflicts

### Performance Issues

#### "High memory usage"

**Reduce event buffering:**
```json
{
  "eventBuffer": {
    "enabled": false
  }
}
```

**Optimize event filtering:**
- Disable unused event types
- Reduce event data payload size
- Implement client-side filtering

#### "Slow command execution"

**Check:**
- Network latency to Bcon server
- Command complexity and argument validation
- Server TPS (may affect command processing)

### Platform-Specific Issues

#### Fabric Issues

**"Fabric API not found"**
- Download and install Fabric API mod
- Ensure compatible version with your Minecraft version

**"Mod not loading"**
- Check `fabric.mod.json` for version compatibility
- Verify Java version (21+ required)
- Check for mod conflicts in logs

#### Paper Issues

**"Plugin failed to enable"**
- Verify `plugin.yml` configuration
- Check Paper version compatibility  
- Review plugin dependencies

**"Commands not registering"**
- Paper may cache command registration
- Restart server after configuration changes
- Check for command name conflicts

#### Folia Issues

**"Threading errors"**
- Folia uses regionized threading
- Some operations must run on specific threads
- Check for "not on main thread" errors

**"Scheduler issues"**
- Plugin automatically detects Folia
- Uses region schedulers when available
- Falls back to Paper behavior if needed

### Log Analysis

#### Enable Detailed Logging
```json
{
  "logging": {
    "level": "DEBUG",
    "events": true,
    "commands": true,
    "websocket": true
  }
}
```

#### Important Log Patterns

**Successful connection:**
```
[INFO] Bcon Adapter: Successfully connected to Bcon server
[INFO] Bcon Adapter: Authenticated as server 'my_minecraft_server'
```

**Event processing:**
```
[DEBUG] Sending event: player_joined (playerId: uuid-here)
[DEBUG] Event sent successfully in 45ms
```

**Command execution:**
```
[DEBUG] Received command: execute_command (command: heal PlayerName)
[DEBUG] Command executed successfully: TargetPlayer has been healed
```

## Advanced Configuration

### Custom Event Filtering

```json
{
  "eventFilters": {
    "enabled": true,
    "includeEvents": [
      "player_joined", 
      "player_left",
      "player_chat",
      "custom_command_executed"
    ],
    "excludeEvents": [
      "player_break_block_before",
      "redstone_update"
    ]
  }
}
```

### Rate Limiting

```json
{
  "rateLimits": {
    "eventsPerSecond": 100,
    "commandsPerMinute": 60,
    "burstEvents": 200
  }
}
```

### Security Settings

```json
{
  "security": {
    "requireSecureConnection": false,
    "validateOrigin": true,
    "allowedCommands": ["heal", "teleport"],
    "restrictedCommands": ["stop", "reload"]
  }
}
```

## Support

- **Documentation**: [Complete API Reference](README.md)
- **Issues**: [GitHub Issues](https://github.com/your-org/bcon/issues)
- **Discord**: [Community Support](https://discord.gg/bcon)
- **Wiki**: [Community Wiki](https://wiki.bcon.example.com)

For additional help, include the following in your support request:
- Platform (Fabric/Paper/Folia)
- Minecraft and Java versions
- Adapter and Bcon server versions
- Relevant log excerpts
- Configuration file (remove sensitive tokens)