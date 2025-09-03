
  1. Connection

  The client initiates a WebSocket connection to the server URL specified in its configuration
  (config.getServerUrl()).

   * Authentication Headers:
       * Authorization: Bearer <jwt_token>
       * Origin: <origin_url> (e.g., http://localhost:3000)

   * Disconnection Handling:
       * If the connection is lost, the client attempts to reconnect every 1 second (RECONNECT_DELAY).
       * If strictMode is enabled in the config and the connection is lost, the Minecraft server will
         shut down.

  ---

  2. General Message Structure

  ##### Client -> Server

  All messages sent from the Fabric mod to the server follow this structure:

   1 {
   2   "eventType": "event_name_here",
   3   "data": { ... }
   4 }

   * eventType (String): The name of the event that occurred in Minecraft.
   * data (JSON Object): A payload containing details about the event. Can be null.

  ##### Server -> Client

  All messages sent from the server to the Fabric mod follow this structure:

   1 {
   2   "id": "unique_message_id",
   3   "type": "action_type_here",
   4   "data": { ... }
   5 }

   * id (String): A unique identifier for the request, which will be included in the client's response.
   * type (String): The command or action the mod should perform.
   * data (JSON Object | String): Payload for the command.

  ##### Client -> Server (Response)

  When the client responds to a server message, it uses this format:

   1 {
   2   "id": "unique_message_id_from_request",
   3   "result": "...",
   4   "error": "..."
   5 }

   * id (String): The same ID from the incoming server message.
   * result (String): The output or success message from the executed command.
   * error (String, optional): An error message if the command failed.

  ---

  3. Client -> Server Events (`eventType`)

  Here is a complete list of events the Fabric mod sends to the server.

  ##### Server Events

   * server_starting: Server is starting. data is null.
   * server_started: Server has started. data is null.
   * server_stopping: Server is stopping. data is null.
   * server_stopped: Server has stopped. data is null.
   * server_tick_start: A new server tick is beginning. data is null.
   * server_tick_end: The server tick has ended. data is null.
   * server_before_save: Server is about to save the world. data is null.
   * server_after_save: Server has finished saving the world. data is null.
   * data_pack_reload_start: Data pack reload is starting. data is null.
   * data_pack_reload_end: Data pack reload has finished.
       * data: { "success": boolean }
   * data_pack_sync: A player's data pack contents are being synced.
       * data: { "playerId": string, "playerName": string, "joined": boolean }

  ##### Player Events

   * player_joined: A player has joined the server.
       * data: { "playerId": string, "playerName": string, "x": double, "y": double, "z": double,
         "dimension": string }
   * player_left: A player has left the server.
       * data: (Same as player_joined)
   * player_respawned: A player has respawned.
       * data: { "playerId": string, "playerName": string, "alive": boolean, "x": double, "y": double,
         "z": double, "dimension": string }
   * player_death: A player has died.
       * data: { "playerId": string, "playerName": string, ..., "deathMessage": string, "attackerId":
         string (optional), "attackerType": string (optional) }
   * player_chat: A player sent a chat message.
       * data: { "playerId": string, "playerName": string, "message": string }
   * player_connection_init: A player's connection is being initialized.
       * data: { "playerId": string, "playerName": string }
   * player_break_block_before: A player is about to break a block.
       * data: { "playerId": string, "x": int, "y": int, "z": int, "block": string }
   * player_break_block_after: A player has broken a block.
       * data: (Same as player_break_block_before)
   * player_break_block_canceled: A player's block breaking was canceled.
       * data: (Same as player_break_block_before)
   * player_attack_block: A player attacks (left-clicks) a block.
       * data: (Same as player_break_block_before)
   * player_use_block: A player uses (right-clicks) a block.
       * data: (Same as player_break_block_before)
   * player_use_item: A player uses an item.
       * data: { "playerId": string, "item": string, "count": int }
   * player_attack_entity: A player attacks an entity.
       * data: { "playerId": string, "entityId": string, "entityType": string }
   * player_use_entity: A player uses (interacts with) an entity.
       * data: (Same as player_attack_entity)

  ##### Entity Events

   * entity_death: A non-player living entity has died.
       * data: { "killedEntity": { ...entity... }, "killer": { ...entity... } (optional), "deathMessage":
         string (optional) }
   * entity_elytra_check: An entity attempts to use an elytra.
       * data: { "entityId": string, "entityType": string, "x": double, "y": double, "z": double }
   * entity_changed_world: An entity moved to a different dimension.
       * data: { "entityId": string, "entityType": string, "originalWorld": string, "newWorld": string,
         "x": double, "y": double, "z": double }
   * entity_start_sleeping: An entity starts sleeping.
       * data: { "entityId": string, "entityType": string, "x": int, "y": int, "z": int, "dimension":
         string }
   * entity_stop_sleeping: An entity stops sleeping.
       * data: (Same as entity_start_sleeping)

  ##### Container & Inventory Events

   * container_interaction_start: Player opens a container (Chest, Barrel, etc.).
       * data: { "playerId": string, "playerName": string, "blockType": string, "x": int, "y": int, "z":
         int, "dimension": string }
   * container_interaction_end: Player closes a container.
       * data: (Same as container_interaction_start)
   * inventory_slot_click: Player clicks a slot in an inventory.
       * data: { "playerId": string, "playerName": string, "inventory": [ { "item": string, "count": int,
         "damage": int, "slot": int } ] }
   * item_dropped: Player drops an item.
       * data: { "playerId": string, "item": { "item": string, "count": int, "damage": int } }

  ##### World & Misc. Events

   * world_tick_start / world_tick_end: A world tick starts/ends.
       * data: { "dimensionKey": string, "time": long, "difficultyLevel": string }
   * world_load / world_unload: A world is loaded/unloaded.
       * data: { "dimensionKey": string, "time": long, "difficultyLevel": string }
   * command_message: A command is executed.
       * data: { "playerId": string (optional), "playerName": string (optional), "message": string }
   * projectile_kill: A projectile killed an entity.
       * data: { "projectileType": string, "ownerId": string (optional), "ownerType": string (optional),
         "target": { ...entity... } }
   * advancement_complete: A player completes an advancement.
       * data: { "playerId": string, "playerName": string, "advancementId": string, "title": string }
   * experience_update: A player's experience changes.
       * data: { "playerId": string, "level": int, "progress": int }
   * merchant_interaction: A player interacts with a villager/merchant.
       * data: { "playerId": string, "merchantId": string, "merchantType": string }
   * weather_update: The weather changes in a world.
       * data: { "dimension": string, "isRaining": boolean, "isThundering": boolean, "rainGradient":
         float }
   * redstone_update: A redstone component's state changes.
       * data: { "x": int, "y": int, "z": int, "power": int }

  ---

  4. Server -> Client Actions (`type`)

  Here is a complete list of actions the server can request from the Fabric mod.








   * command: Executes a command in the server console.
       * data (String): The command to execute (e.g., "say Hello World").
   * setblock: Sets a block at a specific location.
       * data (JSON Object): { "x": int, "y": int, "z": int, "block": "minecraft:stone", "world":
         "minecraft:overworld" }
   * chat: Broadcasts a message to all players.
       * data (String): The message to broadcast.
   * bluemap: Executes a command related to the BlueMap plugin.
       * data (JSON Object): The specific marker command payload.
   * register_command: Registers a new dynamic command.
       * data (JSON Object): The command definition.
   * unregister_command: Unregisters a dynamic command.
       * data (String): The name of the command to unregister.
   * clear_commands: Clears all registered dynamic commands.
       * data: (Not used).
