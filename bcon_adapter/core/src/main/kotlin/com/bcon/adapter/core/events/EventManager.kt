package com.bcon.adapter.core.events

import com.bcon.adapter.core.BconAdapter
import com.google.gson.JsonObject

/**
 * Event management system for Bcon adapter
 * Provides standardized event data serialization and dispatch
 */
class EventManager(private val adapter: BconAdapter) {
    
    private val logger = adapter.logger
    
    // Server Events
    
    fun onServerStarting() {
        logger.info("Server starting")
        adapter.sendEvent("server_starting", null)
    }
    
    fun onServerStarted() {
        logger.info("Server started")
        adapter.sendEvent("server_started", null)
    }
    
    fun onServerStopping() {
        logger.info("Server stopping")
        adapter.sendEvent("server_stopping", null)
    }
    
    fun onServerStopped() {
        logger.info("Server stopped")
        adapter.sendEvent("server_stopped", null)
    }
    
    fun onServerBeforeSave() {
        adapter.sendEvent("server_before_save", null)
    }
    
    fun onServerAfterSave() {
        adapter.sendEvent("server_after_save", null)
    }
    
    fun onDataPackReloadStart() {
        adapter.sendEvent("data_pack_reload_start", null)
    }
    
    fun onDataPackReloadEnd(success: Boolean) {
        val data = JsonObject().apply {
            addProperty("success", success)
        }
        adapter.sendEvent("data_pack_reload_end", data)
    }
    
    // Player Events
    
    fun onPlayerJoined(player: PlayerData) {
        val data = serializePlayer(player)
        adapter.sendEvent("player_joined", data)
        logger.info("Player joined: ${player.name}")
    }
    
    fun onPlayerLeft(player: PlayerData) {
        val data = serializePlayer(player)
        adapter.sendEvent("player_left", data)
        logger.info("Player left: ${player.name}")
    }
    
    fun onPlayerConnectionInit(player: PlayerData) {
        val data = JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("playerName", player.name)
        }
        adapter.sendEvent("player_connection_init", data)
    }
    
    fun onPlayerRespawned(oldPlayer: PlayerData, newPlayer: PlayerData, alive: Boolean) {
        val data = JsonObject().apply {
            addProperty("playerId", newPlayer.uuid)
            addProperty("playerName", newPlayer.name)
            addProperty("alive", alive)
            addProperty("x", newPlayer.location.x)
            addProperty("y", newPlayer.location.y)
            addProperty("z", newPlayer.location.z)
            addProperty("dimension", newPlayer.location.dimension)
        }
        adapter.sendEvent("player_respawned", data)
    }
    
    fun onPlayerDeath(player: PlayerData, deathMessage: String?, attacker: EntityData?) {
        val data = serializePlayer(player).apply {
            deathMessage?.let { addProperty("deathMessage", it) }
            attacker?.let { 
                addProperty("attackerId", it.uuid)
                addProperty("attackerType", it.type)
            }
        }
        adapter.sendEvent("player_death", data)
        logger.info("Player death: ${player.name}")
    }
    
    fun onPlayerBreakBlockBefore(player: PlayerData, block: BlockData) {
        val data = serializeBlockEvent(player, block)
        adapter.sendEvent("player_break_block_before", data)
    }
    
    fun onPlayerBreakBlockAfter(player: PlayerData, block: BlockData) {
        val data = serializeBlockEvent(player, block)
        adapter.sendEvent("player_break_block_after", data)
    }
    
    fun onPlayerChat(player: PlayerData, message: String) {
        val data = JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("playerName", player.name)
            addProperty("message", message)
        }
        adapter.sendEvent("player_chat", data)
    }
    
    // Entity Events
    
    fun onEntityDeath(killer: EntityData?, killed: EntityData, deathMessage: String?) {
        val data = JsonObject().apply {
            add("killedEntity", serializeEntity(killed))
            killer?.let { add("killer", serializeEntity(it)) }
            deathMessage?.let { addProperty("deathMessage", it) }
        }
        adapter.sendEvent("entity_death", data)
    }
    
    fun onProjectileKill(projectile: ProjectileData, target: EntityData) {
        val data = JsonObject().apply {
            addProperty("projectileType", projectile.type)
            projectile.owner?.let {
                addProperty("ownerId", it.uuid)
                addProperty("ownerType", it.type)
            }
            add("target", serializeEntity(target))
        }
        adapter.sendEvent("projectile_kill", data)
    }
    
    // World Events
    
    fun onWorldLoad(world: WorldData) {
        val data = serializeWorld(world)
        adapter.sendEvent("world_load", data)
        logger.info("World loaded: ${world.name}")
    }
    
    fun onWorldUnload(world: WorldData) {
        val data = serializeWorld(world)
        adapter.sendEvent("world_unload", data)
        logger.info("World unloaded: ${world.name}")
    }
    
    // Interaction Events
    
    fun onMerchantInteraction(player: PlayerData, merchant: EntityData) {
        val data = JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("merchantId", merchant.uuid)
            addProperty("merchantType", merchant.type)
        }
        adapter.sendEvent("merchant_interaction", data)
    }
    
    fun onRedstoneUpdate(location: Location, power: Int) {
        val data = JsonObject().apply {
            addProperty("x", location.x)
            addProperty("y", location.y)
            addProperty("z", location.z)
            addProperty("dimension", location.dimension)
            addProperty("power", power)
        }
        adapter.sendEvent("redstone_update", data)
    }
    
    // Advancement Events
    
    fun onAdvancementComplete(player: PlayerData, advancement: AdvancementData) {
        val data = JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("playerName", player.name)
            addProperty("advancementId", advancement.id)
            addProperty("title", advancement.title)
        }
        adapter.sendEvent("advancement_complete", data)
    }
    
    // Command Events
    
    fun onCommandMessage(sender: String, message: String) {
        val data = JsonObject().apply {
            addProperty("sender", sender)
            addProperty("message", message)
        }
        adapter.sendEvent("command_message", data)
    }
    
    // Serialization helpers
    
    private fun serializePlayer(player: PlayerData): JsonObject {
        return JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("playerName", player.name)
            addProperty("x", player.location.x)
            addProperty("y", player.location.y)
            addProperty("z", player.location.z)
            addProperty("dimension", player.location.dimension)
            addProperty("health", player.health)
            addProperty("maxHealth", player.maxHealth)
            addProperty("level", player.level)
            addProperty("gameMode", player.gameMode)
        }
    }
    
    private fun serializeEntity(entity: EntityData): JsonObject {
        return JsonObject().apply {
            addProperty("entityId", entity.uuid)
            addProperty("entityType", entity.type)
            addProperty("x", entity.location.x)
            addProperty("y", entity.location.y)
            addProperty("z", entity.location.z)
            addProperty("dimension", entity.location.dimension)
            entity.name?.let { addProperty("name", it) }
        }
    }
    
    private fun serializeWorld(world: WorldData): JsonObject {
        return JsonObject().apply {
            addProperty("worldName", world.name)
            addProperty("dimensionKey", world.dimensionKey)
            addProperty("time", world.time)
            addProperty("difficultyLevel", world.difficulty)
            addProperty("weather", world.weather)
            addProperty("thundering", world.thundering)
        }
    }
    
    private fun serializeBlockEvent(player: PlayerData, block: BlockData): JsonObject {
        return JsonObject().apply {
            addProperty("playerId", player.uuid)
            addProperty("playerName", player.name)
            addProperty("x", block.location.x)
            addProperty("y", block.location.y)
            addProperty("z", block.location.z)
            addProperty("dimension", block.location.dimension)
            addProperty("blockType", block.type)
            addProperty("blockData", block.data)
        }
    }
}

// Data classes for event system

data class PlayerData(
    val uuid: String,
    val name: String,
    val location: Location,
    val health: Double = 20.0,
    val maxHealth: Double = 20.0,
    val level: Int = 0,
    val gameMode: String = "SURVIVAL"
)

data class EntityData(
    val uuid: String,
    val type: String,
    val location: Location,
    val name: String? = null
)

data class ProjectileData(
    val type: String,
    val location: Location,
    val owner: EntityData? = null
)

data class WorldData(
    val name: String,
    val dimensionKey: String,
    val time: Long,
    val difficulty: String,
    val weather: String = "CLEAR",
    val thundering: Boolean = false
)

data class BlockData(
    val type: String,
    val location: Location,
    val data: String? = null
)

data class AdvancementData(
    val id: String,
    val title: String,
    val description: String = ""
)

data class Location(
    val x: Double,
    val y: Double,
    val z: Double,
    val dimension: String,
    val yaw: Float = 0f,
    val pitch: Float = 0f
)