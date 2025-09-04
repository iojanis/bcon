# Bcon Adapter Events Documentation

This document provides comprehensive documentation for all events supported by the new Bcon Adapter system, which has been successfully ported from the old Fabric mod to support Fabric, Paper, and Folia platforms.

## Architecture Overview

The new Bcon Adapter uses a multi-layered architecture:

- **Core Module**: Platform-agnostic event management, WebSocket communication, and command handling
- **Platform Modules**: Fabric, Paper, and Folia-specific implementations
- **Optional Integrations**: BlueMap support for world visualization

## Event Categories

### Server Events

#### `server_starting`
- **Description**: Fired when the server begins the startup process
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `server_started`
- **Description**: Fired when the server has completed startup and is ready
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `server_stopping`
- **Description**: Fired when the server begins the shutdown process
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `server_stopped`
- **Description**: Fired when the server has completely shut down
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `server_before_save`
- **Description**: Fired before the server saves world data
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `server_after_save`
- **Description**: Fired after the server has saved world data
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `data_pack_reload_start`
- **Description**: Fired when data pack reloading begins
- **Data**: None (null)
- **Platforms**: All (Fabric, Paper, Folia)

#### `data_pack_reload_end`
- **Description**: Fired when data pack reloading completes
- **Data**: 
  ```json
  {
    "success": boolean
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

### Player Events

#### `player_joined`
- **Description**: Fired when a player successfully joins the server
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "x": double,
    "y": double,
    "z": double,
    "dimension": "string",
    "health": double,
    "maxHealth": double,
    "level": int,
    "gameMode": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_left`
- **Description**: Fired when a player leaves the server
- **Data**: Same as `player_joined`
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_connection_init`
- **Description**: Fired during initial player connection setup
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_respawned`
- **Description**: Fired when a player respawns after death
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "alive": boolean,
    "x": double,
    "y": double,
    "z": double,
    "dimension": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_death`
- **Description**: Fired when a player dies
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "x": double,
    "y": double,
    "z": double,
    "dimension": "string",
    "health": double,
    "maxHealth": double,
    "level": int,
    "gameMode": "string",
    "deathMessage": "string (optional)",
    "attackerId": "string (optional)",
    "attackerType": "string (optional)"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_break_block_before`
- **Description**: Fired before a player breaks a block (can be cancelled)
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "x": double,
    "y": double,
    "z": double,
    "dimension": "string",
    "blockType": "string",
    "blockData": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_break_block_after`
- **Description**: Fired after a player successfully breaks a block
- **Data**: Same as `player_break_block_before`
- **Platforms**: All (Fabric, Paper, Folia)

#### `player_chat`
- **Description**: Fired when a player sends a chat message
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "message": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

### Entity Events

#### `entity_death`
- **Description**: Fired when a non-player entity dies
- **Data**:
  ```json
  {
    "killedEntity": {
      "entityId": "string (UUID)",
      "entityType": "string",
      "x": double,
      "y": double,
      "z": double,
      "dimension": "string",
      "name": "string (optional)"
    },
    "killer": {
      "entityId": "string (UUID)",
      "entityType": "string",
      "x": double,
      "y": double,
      "z": double,
      "dimension": "string",
      "name": "string (optional)"
    },
    "deathMessage": "string (optional)"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `projectile_kill`
- **Description**: Fired when a projectile kills an entity
- **Data**:
  ```json
  {
    "projectileType": "string",
    "ownerId": "string (UUID, optional)",
    "ownerType": "string (optional)",
    "target": {
      "entityId": "string (UUID)",
      "entityType": "string",
      "x": double,
      "y": double,
      "z": double,
      "dimension": "string",
      "name": "string (optional)"
    }
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

### World Events

#### `world_load`
- **Description**: Fired when a world is loaded
- **Data**:
  ```json
  {
    "worldName": "string",
    "dimensionKey": "string",
    "time": long,
    "difficultyLevel": "string",
    "weather": "string",
    "thundering": boolean
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `world_unload`
- **Description**: Fired when a world is unloaded
- **Data**: Same as `world_load`
- **Platforms**: All (Fabric, Paper, Folia)

### Interaction Events

#### `merchant_interaction`
- **Description**: Fired when a player interacts with a merchant/villager
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "merchantId": "string (UUID)",
    "merchantType": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `redstone_update`
- **Description**: Fired when redstone power changes
- **Data**:
  ```json
  {
    "x": double,
    "y": double,
    "z": double,
    "dimension": "string",
    "power": int
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

### Advancement Events

#### `advancement_complete`
- **Description**: Fired when a player completes an advancement
- **Data**:
  ```json
  {
    "playerId": "string (UUID)",
    "playerName": "string",
    "advancementId": "string",
    "title": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

### Command Events

#### `command_message`
- **Description**: Fired when a command is executed
- **Data**:
  ```json
  {
    "sender": "string",
    "message": "string"
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

#### `custom_command_executed`
- **Description**: Fired when a dynamic custom command is executed
- **Data**:
  ```json
  {
    "command": "string",
    "subcommand": "string (optional)",
    "sender": "string",
    "senderType": "string (player/console/command_block)",
    "timestamp": long,
    "arguments": {
      "key": "value"
    }
  }
  ```
- **Platforms**: All (Fabric, Paper, Folia)

## Platform-Specific Features

### Fabric
- Uses Fabric API event system
- Brigadier command integration
- Mod loader compatibility

### Paper
- Uses Bukkit/Paper event system
- Plugin architecture
- Enhanced performance optimizations

### Folia
- Thread-safe event handling using region schedulers
- Global region scheduler for cross-region operations
- Automatic fallback to Paper behavior on non-Folia servers

## BlueMap Integration

When BlueMap is available and enabled, the adapter supports additional marker management commands:

### BlueMap Commands (via WebSocket)
- `add_marker`: Add a marker to the map
- `remove_marker`: Remove a marker by ID
- `update_marker`: Update an existing marker
- `list_markers`: List all active markers
- `clear_markers`: Remove all markers
- `get_marker`: Get details for a specific marker

### BlueMap Marker Data
```json
{
  "id": "string",
  "world": "string",
  "x": double,
  "y": double,
  "z": double,
  "label": "string",
  "detail": "string (optional)",
  "icon": "string",
  "anchor": {
    "x": int,
    "y": int
  }
}
```

## Configuration

The adapter uses a JSON configuration file located at `plugins/bcon/config.json`:

```json
{
  "jwtToken": "YOUR_JWT_TOKEN_HERE",
  "serverUrl": "ws://localhost:8082",
  "serverId": "minecraft_server_1",
  "serverName": "Minecraft Server",
  "strictMode": false,
  "reconnectDelayMs": 5000,
  "enableBlueMap": true,
  "heartbeatIntervalMs": 30000,
  "connectionTimeoutMs": 10000
}
```

## WebSocket Commands

The adapter responds to the following WebSocket commands from the Bcon server:

- `command`: Execute a server command
- `chat`: Broadcast a message to all players
- `register_command`: Register a new dynamic command
- `unregister_command`: Remove a dynamic command
- `clear_commands`: Remove all dynamic commands
- `bluemap`: Execute BlueMap marker operations

## Dynamic Command System

The improved dynamic command system supports:

- Complex argument types (string, integer, double, boolean, player, world, item, location, enum)
- Subcommands with their own argument sets
- Argument validation and suggestions
- Tab completion
- Permission checking
- Command aliases
- Built-in response handling

### Command Definition Example
```json
{
  "name": "teleport",
  "description": "Teleport players",
  "usage": "/teleport <player> <location>",
  "permission": "bcon.teleport",
  "aliases": ["tp"],
  "arguments": [
    {
      "name": "player",
      "type": "player",
      "required": true,
      "suggestions": []
    },
    {
      "name": "location",
      "type": "location",
      "required": true
    }
  ],
  "responses": {
    "success": "Player teleported successfully"
  }
}
```

## Migration Notes

This new adapter system replaces the old Fabric-only mod with:

- **Multi-platform support**: Works with Fabric, Paper, and Folia
- **Improved architecture**: Better separation of concerns and maintainability
- **Enhanced command system**: More robust dynamic command handling
- **Optional integrations**: BlueMap support is now optional and gracefully handled
- **Better error handling**: More resilient WebSocket communication and reconnection logic
- **Thread safety**: Proper handling of Folia's regionized threading model

All events from the original Fabric mod have been successfully ported and are available across all supported platforms.