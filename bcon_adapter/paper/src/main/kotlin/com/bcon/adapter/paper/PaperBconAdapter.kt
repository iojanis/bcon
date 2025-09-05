package com.bcon.adapter.paper

import com.bcon.adapter.core.BconAdapter
import com.bcon.adapter.core.events.*
import com.bcon.adapter.paper.commands.PaperCommandManager
import com.bcon.adapter.paper.integration.PaperBlueMapIntegration
import com.bcon.adapter.paper.logging.PaperBconLogger
import com.google.gson.JsonObject
import org.bukkit.Bukkit
import org.bukkit.Server
import org.bukkit.World
import org.bukkit.advancement.Advancement
import org.bukkit.block.Block
import org.bukkit.entity.Entity
import org.bukkit.entity.LivingEntity
import org.bukkit.entity.Player
import org.bukkit.entity.Projectile
import org.bukkit.event.EventHandler
import org.bukkit.event.EventPriority
import org.bukkit.event.Listener
import org.bukkit.event.block.BlockBreakEvent
import org.bukkit.event.entity.*
import org.bukkit.event.inventory.*
import org.bukkit.event.player.*
import org.bukkit.event.server.ServerLoadEvent
import org.bukkit.event.world.WorldLoadEvent
import org.bukkit.event.world.WorldUnloadEvent
import org.bukkit.inventory.ItemStack
import org.bukkit.entity.FishHook
import org.bukkit.entity.Animals
import org.bukkit.plugin.java.JavaPlugin

/**
 * Paper/Bukkit implementation of the Bcon adapter
 * Handles Paper-specific event registration and server integration
 */
class PaperBconAdapter : JavaPlugin(), Listener {
    
    private val adapter = object : BconAdapter() {
        override val logger = PaperBconLogger(this@PaperBconAdapter)
        
        override fun onInitialize() {
            setupPaperComponents()
        }
        
        override fun onShutdown() {
            shutdownPaperComponents()
        }
        
        override fun registerEvents() {
            registerPaperEvents()
        }
        
        override fun executeCommand(command: String): String {
            return executeServerCommand(command)
        }
        
        override fun broadcastMessage(message: String): String {
            return broadcastServerMessage(message)
        }
        
        override fun getServerInstance(): Any = server
        
        override fun isServerRunning(): Boolean = !server.isStopping
        
        override fun getServerInfo(): JsonObject {
            return JsonObject().apply {
                addProperty("platform", "Paper")
                addProperty("running", isServerRunning())
                addProperty("version", server.version)
                addProperty("bukkitVersion", server.bukkitVersion)
                addProperty("playerCount", server.onlinePlayers.size)
                addProperty("maxPlayers", server.maxPlayers)
                addProperty("worldCount", server.worlds.size)
                addProperty("pluginCount", server.pluginManager.plugins.size)
            }
        }
        
        override fun handleStrictModeFailure() {
            logger.severe("Bcon connection failed in strict mode - shutting down server")
            
            // Schedule server shutdown on the main thread
            org.bukkit.Bukkit.getScheduler().runTask(this@PaperBconAdapter, Runnable {
                logger.severe("Executing emergency server shutdown due to Bcon connection failure")
                server.shutdown()
            })
        }
    }
    
    private var paperCommandManager: PaperCommandManager? = null
    private var paperBlueMapIntegration: PaperBlueMapIntegration? = null
    
    override fun onEnable() {
        super.getLogger().info("Enabling Paper Bcon Adapter")
        adapter.initialize()
    }
    
    override fun onDisable() {
        super.getLogger().info("Disabling Paper Bcon Adapter")
        adapter.shutdown()
    }
    
    private fun setupPaperComponents() {
        super.getLogger().info("Setting up Paper-specific components")
        
        // Initialize Paper-specific command manager
        paperCommandManager = PaperCommandManager(this, adapter.commandManager)
        
        // Initialize Paper-specific BlueMap integration if available
        try {
            if (adapter.config.enableBlueMap) {
                paperBlueMapIntegration = PaperBlueMapIntegration(this)
                super.getLogger().info("Paper BlueMap integration enabled")
            }
        } catch (e: Exception) {
            super.getLogger().info("BlueMap not available for Paper, integration disabled")
        }
    }
    
    private fun shutdownPaperComponents() {
        super.getLogger().info("Shutting down Paper Bcon Adapter")
        paperBlueMapIntegration?.shutdown()
        paperCommandManager?.shutdown()
    }
    
    private fun registerPaperEvents() {
        super.getLogger().info("Registering Paper events")
        
        // Register this class as event listener
        server.pluginManager.registerEvents(this, this)
        
        // Register server lifecycle events
        adapter.eventManager.onServerStarted()
        
        super.getLogger().info("Paper events registered successfully")
    }
    
    // Event Handlers
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerJoin(event: PlayerJoinEvent) {
        val playerData = createPlayerData(event.player)
        adapter.eventManager.onPlayerJoined(playerData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerQuit(event: PlayerQuitEvent) {
        val playerData = createPlayerData(event.player)
        adapter.eventManager.onPlayerLeft(playerData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerLogin(event: PlayerLoginEvent) {
        if (event.result == PlayerLoginEvent.Result.ALLOWED) {
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerConnectionInit(playerData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerRespawn(event: PlayerRespawnEvent) {
        val playerData = createPlayerData(event.player)
        adapter.eventManager.onPlayerRespawned(playerData, playerData, true)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerDeath(event: PlayerDeathEvent) {
        val playerData = createPlayerData(event.entity)
        val deathMessage = event.deathMessage()?.toString()
        val killer = event.entity.killer
        val attackerData = killer?.let { createEntityData(it) }
        
        adapter.eventManager.onPlayerDeath(playerData, deathMessage, attackerData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityDeath(event: EntityDeathEvent) {
        if (event.entity !is Player) {
            val killedEntity = createEntityData(event.entity)
            val killer = event.entity.killer
            val killerData = killer?.let { createEntityData(it) }
            val deathMessage = "Entity death"
            
            adapter.eventManager.onEntityDeath(killerData, killedEntity, deathMessage)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onBlockBreak(event: BlockBreakEvent) {
        if (!event.isCancelled) {
            val playerData = createPlayerData(event.player)
            val blockData = createBlockData(event.block)
            adapter.eventManager.onPlayerBreakBlockAfter(playerData, blockData)
        }
    }
    
    @EventHandler(priority = EventPriority.HIGH)
    fun onBlockBreakBefore(event: BlockBreakEvent) {
        val playerData = createPlayerData(event.player)
        val blockData = createBlockData(event.block)
        adapter.eventManager.onPlayerBreakBlockBefore(playerData, blockData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onAsyncPlayerChat(event: AsyncPlayerChatEvent) {
        if (!event.isCancelled) {
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerChat(playerData, event.message)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onWorldLoad(event: WorldLoadEvent) {
        val worldData = createWorldData(event.world)
        adapter.eventManager.onWorldLoad(worldData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onWorldUnload(event: WorldUnloadEvent) {
        val worldData = createWorldData(event.world)
        adapter.eventManager.onWorldUnload(worldData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerAdvancementDone(event: PlayerAdvancementDoneEvent) {
        val playerData = createPlayerData(event.player)
        val advancementData = createAdvancementData(event.advancement)
        adapter.eventManager.onAdvancementComplete(playerData, advancementData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onServerLoad(event: ServerLoadEvent) {
        when (event.type) {
            ServerLoadEvent.LoadType.STARTUP -> adapter.eventManager.onServerStarted()
            ServerLoadEvent.LoadType.RELOAD -> {
                adapter.eventManager.onDataPackReloadStart()
                adapter.eventManager.onDataPackReloadEnd(true)
            }
        }
    }
    
    // New Event Handlers from Skript Analysis
    
    // Fishing Events
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerFish(event: PlayerFishEvent) {
        if (event.isCancelled) return
        
        val playerData = createPlayerData(event.player)
        val hook = event.hook
        val hookData = createFishHookData(hook)
        
        when (event.state) {
            PlayerFishEvent.State.FISHING -> {
                adapter.eventManager.onPlayerFishingCast(playerData, hookData)
            }
            PlayerFishEvent.State.CAUGHT_FISH -> {
                val fish = event.caught
                if (fish != null) {
                    val fishData = createEntityData(fish)
                    adapter.eventManager.onPlayerFishCaught(playerData, fishData, hookData)
                }
            }
            PlayerFishEvent.State.CAUGHT_ENTITY -> {
                val entity = event.caught
                if (entity != null) {
                    val entityData = createEntityData(entity)
                    adapter.eventManager.onPlayerEntityHooked(playerData, entityData, hookData)
                }
            }
            PlayerFishEvent.State.IN_GROUND -> {
                val location = createLocation(hook.location)
                adapter.eventManager.onPlayerFishingGrounded(playerData, location, hookData)
            }
            PlayerFishEvent.State.FAILED_ATTEMPT -> {
                adapter.eventManager.onPlayerFishEscape(playerData, hookData)
            }
            PlayerFishEvent.State.REEL_IN -> {
                adapter.eventManager.onPlayerFishingReelIn(playerData, hookData)
            }
            PlayerFishEvent.State.BITE -> {
                adapter.eventManager.onPlayerFishBite(playerData, hookData)
            }
            else -> {
                // Handle any other states or LURED if available in Paper
                try {
                    val luredState = PlayerFishEvent.State.valueOf("LURED")
                    if (event.state == luredState) {
                        adapter.eventManager.onPlayerFishLured(playerData, hookData)
                    }
                } catch (e: IllegalArgumentException) {
                    // LURED state not available in this version
                }
            }
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerBucketEntity(event: PlayerBucketEntityEvent) {
        if (event.isCancelled) return
        
        val playerData = createPlayerData(event.player)
        val entity = event.entity
        val entityData = createEntityData(entity)
        val bucketData = createItemData(event.originalBucket)
        
        adapter.eventManager.onPlayerBucketEntity(playerData, entityData, bucketData)
    }
    
    // Breeding Events
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityBreed(event: EntityBreedEvent) {
        if (event.isCancelled) return
        
        val mother = event.mother
        val father = event.father
        val breeder = event.breeder as? Player
        val offspring = event.entity
        
        val motherData = createEntityData(mother)
        val fatherData = createEntityData(father)
        val breederData = breeder?.let { createPlayerData(it) }
        val offspringData = createEntityData(offspring)
        
        adapter.eventManager.onEntityStartBreeding(motherData, fatherData, breederData, offspringData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityEnterLoveMode(event: EntityEnterLoveModeEvent) {
        if (event.isCancelled) return
        
        val entity = event.entity
        val cause = event.humanEntity as? Player
        
        val entityData = createEntityData(entity)
        val causeData = cause?.let { createPlayerData(it) }
        
        adapter.eventManager.onEntityEnterLoveMode(entityData, causeData)
    }
    
    // Enhanced Entity Events
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityDamage(event: EntityDamageEvent) {
        if (event.isCancelled) return
        
        val entity = event.entity
        val entityData = createEntityData(entity)
        val damage = event.damage
        val damageType = event.cause.name
        
        val damageSource = if (event is EntityDamageByEntityEvent) {
            event.damager?.let { createEntityData(it) }
        } else null
        
        adapter.eventManager.onEntityDamage(entityData, damage, damageType, damageSource)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityRegainHealth(event: EntityRegainHealthEvent) {
        if (event.isCancelled) return
        
        val entity = event.entity
        val entityData = createEntityData(entity)
        val healAmount = event.amount
        val healReason = event.regainReason.name
        
        adapter.eventManager.onEntityHeal(entityData, healAmount, healReason)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityMount(event: EntityMountEvent) {
        if (event.isCancelled) return
        
        val rider = event.entity
        val mount = event.mount
        
        val riderData = createEntityData(rider)
        val mountData = createEntityData(mount)
        
        adapter.eventManager.onEntityMount(riderData, mountData)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityDismount(event: EntityDismountEvent) {
        if (event.isCancelled) return
        
        val rider = event.entity
        val mount = event.dismounted
        
        val riderData = createEntityData(rider)
        val mountData = createEntityData(mount)
        
        adapter.eventManager.onEntityDismount(riderData, mountData)
    }
    
    // Furnace Events
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onFurnaceSmelt(event: FurnaceSmeltEvent) {
        if (event.isCancelled) return
        
        val location = createLocation(event.block.location)
        val source = createItemData(event.source)
        val result = createItemData(event.result)
        
        adapter.eventManager.onFurnaceSmelt(location, source, result)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onFurnaceBurn(event: FurnaceBurnEvent) {
        if (event.isCancelled) return
        
        val location = createLocation(event.block.location)
        val fuel = createItemData(event.fuel)
        val burnTime = event.burnTime
        
        adapter.eventManager.onFurnaceBurn(location, fuel, burnTime)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onFurnaceExtract(event: FurnaceExtractEvent) {
        
        val player = event.player
        val location = createLocation(event.block.location)
        val item = ItemStack(event.itemType, event.itemAmount)
        val experience = event.expToDrop
        
        val playerData = createPlayerData(player)
        val itemData = createItemData(item)
        
        adapter.eventManager.onFurnaceExtract(playerData, location, itemData, experience)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onFurnaceStartSmelt(event: FurnaceStartSmeltEvent) {
        
        val location = createLocation(event.block.location)
        val source = createItemData(event.source)
        val totalCookTime = event.totalCookTime
        
        adapter.eventManager.onFurnaceStartSmelt(location, source, totalCookTime)
    }
    
    // Inventory Events
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onInventoryOpen(event: InventoryOpenEvent) {
        if (event.isCancelled) return
        
        val player = event.player as? Player ?: return
        val playerData = createPlayerData(player)
        val inventoryType = event.inventory.type.name
        val location = event.inventory.location?.let { createLocation(it) }
        
        adapter.eventManager.onPlayerInventoryOpen(playerData, inventoryType, location)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onInventoryClose(event: InventoryCloseEvent) {
        val player = event.player as? Player ?: return
        val playerData = createPlayerData(player)
        val inventoryType = event.inventory.type.name
        
        adapter.eventManager.onPlayerInventoryClose(playerData, inventoryType)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerDropItem(event: PlayerDropItemEvent) {
        if (event.isCancelled) return
        
        val playerData = createPlayerData(event.player)
        val itemData = createItemData(event.itemDrop.itemStack)
        val location = createLocation(event.itemDrop.location)
        
        adapter.eventManager.onPlayerItemDrop(playerData, itemData, location)
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerPickupItem(event: PlayerPickupItemEvent) {
        if (event.isCancelled) return
        
        val playerData = createPlayerData(event.player)
        val itemData = createItemData(event.item.itemStack)
        val location = createLocation(event.item.location)
        
        adapter.eventManager.onPlayerItemPickup(playerData, itemData, location)
    }
    
    // Command and server management
    
    private fun executeServerCommand(command: String): String {
        return try {
            val result = Bukkit.dispatchCommand(Bukkit.getConsoleSender(), command)
            if (result) {
                "Command executed successfully"
            } else {
                "Command execution failed"
            }
        } catch (e: Exception) {
            super.getLogger().severe("Failed to execute command '$command': ${e.message}")
            "Failed to execute command: ${e.message}"
        }
    }
    
    private fun broadcastServerMessage(message: String): String {
        return try {
            Bukkit.broadcastMessage(message)
            "Message broadcasted successfully"
        } catch (e: Exception) {
            super.getLogger().severe("Failed to broadcast message: ${e.message}")
            "Failed to broadcast message: ${e.message}"
        }
    }
    
    // Helper methods for data conversion
    
    private fun createPlayerData(player: Player): PlayerData {
        return PlayerData(
            uuid = player.uniqueId.toString(),
            name = player.name,
            location = createLocation(player.location),
            health = player.health,
            maxHealth = player.maxHealth,
            level = player.level,
            gameMode = player.gameMode.name
        )
    }
    
    private fun createEntityData(entity: Entity): EntityData {
        return EntityData(
            uuid = entity.uniqueId.toString(),
            type = entity.type.name,
            location = createLocation(entity.location),
            name = if (entity is LivingEntity) entity.customName else null
        )
    }
    
    private fun createWorldData(world: World): WorldData {
        return WorldData(
            name = world.name,
            dimensionKey = world.environment.name,
            time = world.time,
            difficulty = world.difficulty.name,
            weather = when {
                world.hasStorm() -> "RAIN"
                world.isThundering -> "THUNDER"
                else -> "CLEAR"
            },
            thundering = world.isThundering
        )
    }
    
    private fun createBlockData(block: Block): BlockData {
        return BlockData(
            type = block.type.name,
            location = createLocation(block.location),
            data = block.blockData.asString
        )
    }
    
    private fun createLocation(location: org.bukkit.Location): Location {
        return Location(
            x = location.x,
            y = location.y,
            z = location.z,
            dimension = location.world?.environment?.name ?: "NORMAL",
            yaw = location.yaw,
            pitch = location.pitch
        )
    }
    
    private fun createAdvancementData(advancement: Advancement): AdvancementData {
        val display = advancement.display
        return AdvancementData(
            id = advancement.key.toString(),
            title = display?.title()?.toString() ?: "Unknown",
            description = display?.description()?.toString() ?: ""
        )
    }
    
    private fun createItemData(item: ItemStack): ItemData {
        val meta = item.itemMeta
        return ItemData(
            type = item.type.name,
            amount = item.amount,
            displayName = meta?.displayName()?.toString(),
            lore = meta?.lore()?.map { it.toString() },
            nbt = try {
                item.toString() // Simplified NBT representation
            } catch (e: Exception) {
                null
            }
        )
    }
    
    private fun createFishHookData(hook: FishHook): FishHookData {
        return FishHookData(
            location = createLocation(hook.location),
            inOpenWater = try {
                hook.isInOpenWater
            } catch (e: Exception) {
                false // Fallback for older versions
            },
            waitTime = try {
                hook.waitTime
            } catch (e: Exception) {
                0 // Fallback for older versions
            }
        )
    }
    
}