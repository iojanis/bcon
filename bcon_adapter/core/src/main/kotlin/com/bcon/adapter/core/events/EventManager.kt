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
    
    fun onPlayerRespawned(@Suppress("UNUSED_PARAMETER") oldPlayer: PlayerData, newPlayer: PlayerData, alive: Boolean) {
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
    
    // Fishing Events
    
    fun onPlayerFishingCast(player: PlayerData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
            addProperty("inOpenWater", hook.inOpenWater)
            addProperty("waitTime", hook.waitTime)
        }
        adapter.sendEvent("player_fishing_cast", data)
    }
    
    fun onPlayerFishCaught(player: PlayerData, fish: EntityData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            add("fish", serializeEntity(fish))
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
            addProperty("inOpenWater", hook.inOpenWater)
        }
        adapter.sendEvent("player_fish_caught", data)
    }
    
    fun onPlayerEntityHooked(player: PlayerData, entity: EntityData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            add("hookedEntity", serializeEntity(entity))
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
        }
        adapter.sendEvent("player_entity_hooked", data)
    }
    
    fun onPlayerFishingGrounded(player: PlayerData, location: Location, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("groundX", location.x)
            addProperty("groundY", location.y)
            addProperty("groundZ", location.z)
            addProperty("groundDimension", location.dimension)
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
        }
        adapter.sendEvent("player_fishing_grounded", data)
    }
    
    fun onPlayerFishEscape(player: PlayerData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
        }
        adapter.sendEvent("player_fish_escape", data)
    }
    
    fun onPlayerFishingReelIn(player: PlayerData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
        }
        adapter.sendEvent("player_fishing_reel_in", data)
    }
    
    fun onPlayerFishBite(player: PlayerData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
            addProperty("waitTime", hook.waitTime)
        }
        adapter.sendEvent("player_fish_bite", data)
    }
    
    fun onPlayerFishLured(player: PlayerData, hook: FishHookData) {
        val data = serializePlayer(player).apply {
            addProperty("hookX", hook.location.x)
            addProperty("hookY", hook.location.y)
            addProperty("hookZ", hook.location.z)
            addProperty("hookDimension", hook.location.dimension)
            addProperty("inOpenWater", hook.inOpenWater)
        }
        adapter.sendEvent("player_fish_lured", data)
    }
    
    fun onPlayerBucketEntity(player: PlayerData, entity: EntityData, bucket: ItemData) {
        val data = serializePlayer(player).apply {
            add("entity", serializeEntity(entity))
            add("bucket", serializeItem(bucket))
        }
        adapter.sendEvent("player_bucket_entity", data)
    }
    
    // Breeding Events
    
    fun onEntityStartBreeding(mother: EntityData, father: EntityData, breeder: PlayerData?, offspring: EntityData?) {
        val data = JsonObject().apply {
            add("mother", serializeEntity(mother))
            add("father", serializeEntity(father))
            breeder?.let { add("breeder", serializePlayer(it)) }
            offspring?.let { add("offspring", serializeEntity(it)) }
        }
        adapter.sendEvent("entity_start_breeding", data)
    }
    
    fun onEntityEnterLoveMode(entity: EntityData, cause: PlayerData?) {
        val data = JsonObject().apply {
            add("entity", serializeEntity(entity))
            cause?.let { add("cause", serializePlayer(it)) }
        }
        adapter.sendEvent("entity_enter_love_mode", data)
    }
    
    // Enhanced Entity Events
    
    fun onEntityDamage(entity: EntityData, damage: Double, damageType: String, damageSource: EntityData?) {
        val data = JsonObject().apply {
            add("entity", serializeEntity(entity))
            addProperty("damage", damage)
            addProperty("damageType", damageType)
            damageSource?.let { add("damageSource", serializeEntity(it)) }
        }
        adapter.sendEvent("entity_damage", data)
    }
    
    fun onEntityHeal(entity: EntityData, healAmount: Double, healReason: String) {
        val data = JsonObject().apply {
            add("entity", serializeEntity(entity))
            addProperty("healAmount", healAmount)
            addProperty("healReason", healReason)
        }
        adapter.sendEvent("entity_heal", data)
    }
    
    fun onEntityMount(rider: EntityData, mount: EntityData) {
        val data = JsonObject().apply {
            add("rider", serializeEntity(rider))
            add("mount", serializeEntity(mount))
        }
        adapter.sendEvent("entity_mount", data)
    }
    
    fun onEntityDismount(rider: EntityData, mount: EntityData) {
        val data = JsonObject().apply {
            add("rider", serializeEntity(rider))
            add("mount", serializeEntity(mount))
        }
        adapter.sendEvent("entity_dismount", data)
    }
    
    // Furnace Events
    
    fun onFurnaceSmelt(location: Location, source: ItemData, result: ItemData) {
        val data = JsonObject().apply {
            addProperty("x", location.x)
            addProperty("y", location.y)
            addProperty("z", location.z)
            addProperty("dimension", location.dimension)
            add("source", serializeItem(source))
            add("result", serializeItem(result))
        }
        adapter.sendEvent("furnace_smelt", data)
    }
    
    fun onFurnaceBurn(location: Location, fuel: ItemData, burnTime: Int) {
        val data = JsonObject().apply {
            addProperty("x", location.x)
            addProperty("y", location.y)
            addProperty("z", location.z)
            addProperty("dimension", location.dimension)
            add("fuel", serializeItem(fuel))
            addProperty("burnTime", burnTime)
        }
        adapter.sendEvent("furnace_burn", data)
    }
    
    fun onFurnaceExtract(player: PlayerData, location: Location, item: ItemData, experience: Int) {
        val data = serializePlayer(player).apply {
            addProperty("furnaceX", location.x)
            addProperty("furnaceY", location.y)
            addProperty("furnaceZ", location.z)
            addProperty("furnaceDimension", location.dimension)
            add("item", serializeItem(item))
            addProperty("experience", experience)
        }
        adapter.sendEvent("furnace_extract", data)
    }
    
    fun onFurnaceStartSmelt(location: Location, source: ItemData, totalCookTime: Int) {
        val data = JsonObject().apply {
            addProperty("x", location.x)
            addProperty("y", location.y)
            addProperty("z", location.z)
            addProperty("dimension", location.dimension)
            add("source", serializeItem(source))
            addProperty("totalCookTime", totalCookTime)
        }
        adapter.sendEvent("furnace_start_smelt", data)
    }
    
    // Inventory Events
    
    fun onPlayerInventoryOpen(player: PlayerData, inventoryType: String, location: Location?) {
        val data = serializePlayer(player).apply {
            addProperty("inventoryType", inventoryType)
            location?.let {
                addProperty("inventoryX", it.x)
                addProperty("inventoryY", it.y)
                addProperty("inventoryZ", it.z)
                addProperty("inventoryDimension", it.dimension)
            }
        }
        adapter.sendEvent("player_inventory_open", data)
    }
    
    fun onPlayerInventoryClose(player: PlayerData, inventoryType: String) {
        val data = serializePlayer(player).apply {
            addProperty("inventoryType", inventoryType)
        }
        adapter.sendEvent("player_inventory_close", data)
    }
    
    fun onPlayerItemDrop(player: PlayerData, item: ItemData, location: Location) {
        val data = serializePlayer(player).apply {
            add("item", serializeItem(item))
            addProperty("dropX", location.x)
            addProperty("dropY", location.y)
            addProperty("dropZ", location.z)
            addProperty("dropDimension", location.dimension)
        }
        adapter.sendEvent("player_item_drop", data)
    }
    
    fun onPlayerItemPickup(player: PlayerData, item: ItemData, location: Location) {
        val data = serializePlayer(player).apply {
            add("item", serializeItem(item))
            addProperty("pickupX", location.x)
            addProperty("pickupY", location.y)
            addProperty("pickupZ", location.z)
            addProperty("pickupDimension", location.dimension)
        }
        adapter.sendEvent("player_item_pickup", data)
    }
    
    // Advanced Events (Version Dependent)
    
    fun onLootGenerate(location: Location, lootTable: String, items: List<ItemData>, entity: EntityData?) {
        val data = JsonObject().apply {
            addProperty("x", location.x)
            addProperty("y", location.y)
            addProperty("z", location.z)
            addProperty("dimension", location.dimension)
            addProperty("lootTable", lootTable)
            val itemsArray = com.google.gson.JsonArray().apply {
                items.forEach { add(serializeItem(it)) }
            }
            add("items", itemsArray)
            entity?.let { add("entity", serializeEntity(it)) }
        }
        adapter.sendEvent("loot_generate", data)
    }
    
    fun onPlayerInput(player: PlayerData, keys: Set<String>, inputType: String) {
        val data = serializePlayer(player).apply {
            val keysArray = com.google.gson.JsonArray().apply {
                keys.forEach { add(it) }
            }
            add("keys", keysArray)
            addProperty("inputType", inputType)
        }
        adapter.sendEvent("player_input", data)
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
    
    private fun serializeItem(item: ItemData): JsonObject {
        return JsonObject().apply {
            addProperty("type", item.type)
            addProperty("amount", item.amount)
            item.displayName?.let { addProperty("displayName", it) }
            item.lore?.let { 
                val loreArray = it.toTypedArray()
                add("lore", com.google.gson.JsonArray().apply {
                    loreArray.forEach { loreItem -> add(loreItem) }
                })
            }
            item.nbt?.let { addProperty("nbt", it) }
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

data class ItemData(
    val type: String,
    val amount: Int,
    val displayName: String? = null,
    val lore: List<String>? = null,
    val nbt: String? = null
)

data class FishHookData(
    val location: Location,
    val inOpenWater: Boolean,
    val waitTime: Int
)

data class DamageSourceData(
    val type: String,
    val directEntity: EntityData?,
    val causingEntity: EntityData?,
    val isProjectile: Boolean = false,
    val isMagic: Boolean = false,
    val isExplosion: Boolean = false
)