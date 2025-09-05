# Bcon Adapter: Multi-Platform Minecraft Integration

A comprehensive Minecraft server adapter that connects **Fabric**, **Paper**, and **Folia** servers to the Bcon ecosystem. The adapter provides real-time event streaming, dynamic command execution, and optional BlueMap integration across all supported platforms.

<!-- modrinth_exclude.start -->
### [DOWNLOAD](https://modrinth.com/mod/bcon) | [Plugin Version](https://modrinth.com/plugin/bcon)
<!-- modrinth_exclude.end -->

## 🚀 Features

### ✅ **Multi-Platform Support**
- **Fabric**: Native mod integration with Fabric API
- **Paper/Spigot**: Full Bukkit/Paper plugin support
- **Folia**: Thread-safe implementation for regionized servers

### 🔥 **Comprehensive Event System**
- **20+ Event Types**: Server lifecycle, player actions, world events, entity interactions
- **Real-time Streaming**: Low-latency event delivery to Bcon server
- **Comprehensive Data**: Rich event payloads with all relevant game state information

### ⚡ **Enhanced Command System**
- **Dynamic Commands**: Register commands at runtime via WebSocket
- **Advanced Arguments**: Support for complex argument types, validation, and tab completion
- **Subcommands**: Nested command structures with individual permissions
- **Built-in Responses**: Configurable success/error messages

### 🗺️ **Optional Integrations**
- **BlueMap Support**: Add/remove/update map markers dynamically
- **Graceful Degradation**: Works perfectly without optional dependencies

## Architecture Overview

### Core Module (`core/`)
Platform-agnostic components used by all implementations:

- **`BconAdapter`**: Abstract base class for platform implementations
- **`EventManager`**: Centralized event handling and serialization
- **`DynamicCommandManager`**: Enhanced command system with validation
- **`BconWebSocketClient`**: Robust WebSocket communication with reconnection
- **`BlueMapIntegration`**: Optional map marker management

### Platform Modules

#### Fabric (`fabric/`)
- **`FabricBconAdapter`**: Main mod class implementing `DedicatedServerModInitializer`
- **`FabricCommandManager`**: Brigadier integration for command registration
- **`FabricBlueMapIntegration`**: Fabric-specific BlueMap API calls

#### Paper (`paper/`)
- **`PaperBconAdapter`**: Main plugin class extending `JavaPlugin`
- **`PaperCommandManager`**: Bukkit command system integration
- **`PaperBlueMapIntegration`**: Paper-specific BlueMap plugin integration

#### Folia (`folia/`)  
- **`FoliaBconAdapter`**: Thread-safe implementation using region schedulers
- **`FoliaCommandManager`**: Thread-safe command registration
- **`FoliaBlueMapIntegration`**: Thread-safe BlueMap operations

## Event Types

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

## Installation

### Fabric Servers

1. **Download the Mod**
   - Download from [Modrinth](https://modrinth.com/mod/bcon)
   - Or build from source: `./gradlew fabric:build`

2. **Install**
   - Copy JAR to `mods/` folder
   - Ensure Fabric API is installed

3. **Configure**
   ```json
   // config/bcon/config.json
   {
     "jwtToken": "your_adapter_token_here",
     "serverUrl": "ws://localhost:8082",
     "serverId": "fabric_server_1",
     "serverName": "My Fabric Server",
     "enableBlueMap": true
   }
   ```

### Paper/Spigot Servers

1. **Download the Plugin**
   - Download from [Modrinth](https://modrinth.com/plugin/bcon)
   - Or build from source: `./gradlew paper:build`

2. **Install**
   - Copy JAR to `plugins/` folder
   - Restart server

3. **Configure**
   ```json
   // plugins/bcon/config.json
   {
     "jwtToken": "your_adapter_token_here",
     "serverUrl": "ws://localhost:8082",
     "serverId": "paper_server_1",
     "serverName": "My Paper Server"
   }
   ```

### Folia Servers

Use the same Paper plugin - it automatically detects and supports Folia's regionized threading.

## Dynamic Commands

### Creating Commands

Commands are defined via JSON and registered at runtime:

```json
{
  "name": "teleport",
  "description": "Teleport to coordinates or player",
  "permission": "bcon.teleport",
  "arguments": [
    {
      "name": "target",
      "type": "PLAYER",
      "required": true
    },
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
    }
  ],
  "subcommands": [
    {
      "name": "to",
      "description": "Teleport to another player",
      "arguments": [
        {
          "name": "player",
          "type": "PLAYER",
          "required": true
        }
      ]
    }
  ]
}
```

### Command Registration

Send via WebSocket to register commands:

```json
{
  "eventType": "register_command",
  "data": {
    "server_id": "your_server_id",
    "command": { /* command definition */ }
  }
}
```

### Supported Argument Types

- `STRING`: Text input with optional validation
- `INTEGER`: Whole numbers with min/max bounds
- `DOUBLE`: Decimal numbers with precision control
- `BOOLEAN`: True/false values
- `PLAYER`: Online player selection with tab completion
- `WORLD`: World/dimension selection
- `ITEM`: Item/block type with NBT support
- `LOCATION`: Coordinate input with world context
- `ENUM`: Predefined value selection

## BlueMap Integration

When BlueMap is installed, the adapter can manage map markers:

### Adding Markers
```json
{
  "eventType": "add_marker",
  "data": {
    "server_id": "your_server_id",
    "marker": {
      "id": "spawn_marker",
      "label": "Server Spawn",
      "x": 0.0,
      "y": 64.0,
      "z": 0.0,
      "world": "world",
      "icon": "assets/spawn.svg",
      "anchor": {
        "x": 25,
        "y": 45
      }
    }
  }
}
```

### Marker Management
- **Add Markers**: Dynamic marker creation
- **Update Markers**: Modify existing markers
- **Remove Markers**: Clean marker removal
- **List Markers**: Query existing markers

## Configuration Reference

### Basic Configuration
```json
{
  "jwtToken": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "serverUrl": "ws://localhost:8082",
  "serverId": "minecraft_server_1",
  "serverName": "My Minecraft Server"
}
```

### Advanced Configuration
```json
{
  "jwtToken": "your_token",
  "serverUrl": "ws://localhost:8082", 
  "serverId": "server_id",
  "serverName": "Display Name",
  "enableBlueMap": true,
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

## Building from Source

### Prerequisites
- Java 21+
- Gradle 8.0+

### Build Commands

```bash
# Build all platforms
./gradlew build

# Build specific platform
./gradlew fabric:build    # Fabric mod
./gradlew paper:build     # Paper plugin

# Build with dependencies
./gradlew fabric:buildFatJar    # Includes all dependencies

# Run tests
./gradlew test

# Generate documentation
./gradlew javadoc
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/your-org/bcon.git
cd bcon/bcon_adapter

# Generate development config
./gradlew generateDevConfig

# Run in development (Fabric)
./gradlew fabric:runClient
./gradlew fabric:runServer
```

## API Reference

### Event Data Format

All events follow a consistent structure:

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
    "health": 20.0,
    "maxHealth": 20.0,
    "level": 25,
    "ip": "192.168.1.100",
    "version": "1.20.4"
  },
  "timestamp": 1699123456
}
```

### WebSocket Messages

#### From Server (Commands)
```json
{
  "eventType": "execute_command",
  "data": {
    "command": "give PlayerName diamond 64",
    "server_id": "your_server_id"
  }
}
```

#### To Server (Events)
```json
{
  "eventType": "player_chat",
  "data": {
    "playerId": "uuid-here",
    "playerName": "PlayerName",
    "message": "Hello world!",
    "server_id": "your_server_id"
  }
}
```

## Troubleshooting

### Connection Issues

**Adapter not connecting**
- Verify JWT token validity
- Check server URL and port
- Review firewall settings
- Enable debug logging

**Events not appearing**
- Check WebSocket connection status
- Verify server-side event listeners
- Review rate limiting settings

### Performance

**High memory usage**
- Reduce event buffer size
- Disable unnecessary event types
- Implement event filtering

**Connection drops**
- Increase reconnect interval
- Check network stability
- Review server resources

### Platform Specific

**Fabric Issues**
- Ensure Fabric API is installed
- Check mod compatibility
- Review fabric.mod.json

**Paper Issues**
- Verify plugin.yml configuration
- Check Paper version compatibility
- Review plugin dependencies

**Folia Issues**
- Confirm Folia detection
- Check region scheduler usage
- Verify thread-safe operations

## Support

- **Documentation**: See [GUIDE.md](GUIDE.md) for detailed setup
- **Issues**: [GitHub Issues](https://github.com/your-org/bcon/issues)
- **Discord**: [Community Server](https://discord.gg/bcon)
- **Modrinth**: [Mod Page](https://modrinth.com/mod/bcon)

## License

MIT License - see [LICENSE](../LICENSE) file for details.