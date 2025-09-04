package com.bcon.adapter.folia

import com.bcon.adapter.core.BconAdapter
import com.bcon.adapter.core.events.*
import com.bcon.adapter.folia.commands.FoliaCommandManager
import com.bcon.adapter.folia.integration.FoliaBlueMapIntegration
import com.bcon.adapter.folia.logging.FoliaBconLogger
import com.google.gson.JsonObject
import io.papermc.paper.threadedregions.scheduler.ScheduledTask
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
import org.bukkit.event.entity.EntityDeathEvent
import org.bukkit.event.entity.PlayerDeathEvent
import org.bukkit.event.player.*
import org.bukkit.event.server.ServerLoadEvent
import org.bukkit.event.world.WorldLoadEvent
import org.bukkit.event.world.WorldUnloadEvent
import org.bukkit.plugin.java.JavaPlugin

/**
 * Folia implementation of the Bcon adapter
 * Handles Folia-specific event registration and server integration with region-based threading
 */
class FoliaBconAdapter : JavaPlugin(), Listener {
    
    private val adapter = object : BconAdapter() {
        override val logger = FoliaBconLogger(this@FoliaBconAdapter)
        
        override fun onInitialize() {
            setupFoliaComponents()
        }
        
        override fun onShutdown() {
            shutdownFoliaComponents()
        }
        
        override fun registerEvents() {
            registerFoliaEvents()
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
                addProperty("platform", "Folia")
                addProperty("running", isServerRunning())
                addProperty("version", server.version)
                addProperty("bukkitVersion", server.bukkitVersion)
                addProperty("playerCount", server.onlinePlayers.size)
                addProperty("maxPlayers", server.maxPlayers)
                addProperty("worldCount", server.worlds.size)
                addProperty("pluginCount", server.pluginManager.plugins.size)
                addProperty("regionThreading", isFolia())
            }
        }
        
        override fun handleStrictModeFailure() {
            logger.severe("Bcon connection failed in strict mode - shutting down server")
            
            // Schedule server shutdown on the global scheduler (Folia-compatible)
            scheduleGlobalTask {
                logger.severe("Executing emergency server shutdown due to Bcon connection failure")
                server.shutdown()
            }
        }
    }
    
    private var foliaCommandManager: FoliaCommandManager? = null
    private var foliaBlueMapIntegration: FoliaBlueMapIntegration? = null
    private var scheduledTasks = mutableListOf<ScheduledTask>()
    
    override fun onEnable() {
        super.getLogger().info("Enabling Folia Bcon Adapter")
        
        // Check if this is actually Folia
        if (!isFolia()) {
            super.getLogger().warning("This server is not running Folia! Consider using the Paper adapter instead.")
        }
        
        adapter.initialize()
    }
    
    override fun onDisable() {
        super.getLogger().info("Disabling Folia Bcon Adapter")
        adapter.shutdown()
    }
    
    private fun setupFoliaComponents() {
        super.getLogger().info("Setting up Folia-specific components")
        
        // Initialize Folia-specific command manager
        foliaCommandManager = FoliaCommandManager(this, adapter.commandManager)
        
        // Initialize Folia-specific BlueMap integration if available
        try {
            if (adapter.config.enableBlueMap) {
                foliaBlueMapIntegration = FoliaBlueMapIntegration(this)
                super.getLogger().info("Folia BlueMap integration enabled")
            }
        } catch (e: Exception) {
            super.getLogger().info("BlueMap not available for Folia, integration disabled")
        }
        
        // Schedule heartbeat on global region scheduler
        scheduleHeartbeat()
    }
    
    private fun shutdownFoliaComponents() {
        super.getLogger().info("Shutting down Folia Bcon Adapter")
        
        // Cancel all scheduled tasks
        scheduledTasks.forEach { task ->
            try {
                task.cancel()
            } catch (e: Exception) {
                super.getLogger().warning("Failed to cancel task: ${e.message}")
            }
        }
        scheduledTasks.clear()
        
        foliaBlueMapIntegration?.shutdown()
        foliaCommandManager?.shutdown()
    }
    
    private fun registerFoliaEvents() {
        super.getLogger().info("Registering Folia events")
        
        // Register this class as event listener
        server.pluginManager.registerEvents(this, this)
        
        // Register server lifecycle events
        adapter.eventManager.onServerStarted()
        
        super.getLogger().info("Folia events registered successfully")
    }
    
    /**
     * Check if the server is running Folia
     */
    private fun isFolia(): Boolean {
        return try {
            Class.forName("io.papermc.paper.threadedregions.RegionizedServer")
            true
        } catch (e: ClassNotFoundException) {
            false
        }
    }
    
    /**
     * Schedule heartbeat task using Folia's global region scheduler
     */
    private fun scheduleHeartbeat() {
        try {
            if (isFolia()) {
                // Use Folia's global region scheduler for heartbeat
                val task = Bukkit.getGlobalRegionScheduler().runAtFixedRate(this, { _ ->
                    // Heartbeat logic - handled by WebSocket client
                }, 1L, (adapter.config.heartbeatInterval / 50).toLong()) // Convert ms to ticks
                
                scheduledTasks.add(task)
                super.getLogger().info("Folia heartbeat scheduled using global region scheduler")
            } else {
                // Fallback to regular scheduler
                Bukkit.getScheduler().runTaskTimerAsynchronously(this, Runnable {
                    // Heartbeat logic
                }, 1L, (adapter.config.heartbeatInterval / 50).toLong())
                
                super.getLogger().info("Heartbeat scheduled using regular scheduler")
            }
        } catch (e: Exception) {
            super.getLogger().warning("Failed to schedule heartbeat: ${e.message}")
        }
    }
    
    // Event Handlers (same as Paper but with Folia considerations)
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerJoin(event: PlayerJoinEvent) {
        // Schedule on entity scheduler for the player
        scheduleEntityTask(event.player) {
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerJoined(playerData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerQuit(event: PlayerQuitEvent) {
        // Schedule on entity scheduler for the player
        scheduleEntityTask(event.player) {
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerLeft(playerData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerLogin(event: PlayerLoginEvent) {
        if (event.result == PlayerLoginEvent.Result.ALLOWED) {
            scheduleEntityTask(event.player) {
                val playerData = createPlayerData(event.player)
                adapter.eventManager.onPlayerConnectionInit(playerData)
            }
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerRespawn(event: PlayerRespawnEvent) {
        scheduleEntityTask(event.player) {
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerRespawned(playerData, playerData, true)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerDeath(event: PlayerDeathEvent) {
        scheduleEntityTask(event.entity) {
            val playerData = createPlayerData(event.entity)
            val deathMessage = event.deathMessage()?.toString()
            val killer = event.entity.killer
            val attackerData = killer?.let { createEntityData(it) }
            
            adapter.eventManager.onPlayerDeath(playerData, deathMessage, attackerData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onEntityDeath(event: EntityDeathEvent) {
        if (event.entity !is Player) {
            scheduleEntityTask(event.entity) {
                val killedEntity = createEntityData(event.entity)
                val killer = event.entity.killer
                val killerData = killer?.let { createEntityData(it) }
                val deathMessage = "Entity death"
                
                adapter.eventManager.onEntityDeath(killerData, killedEntity, deathMessage)
            }
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onBlockBreak(event: BlockBreakEvent) {
        if (!event.isCancelled) {
            scheduleRegionTask(event.block.location) {
                val playerData = createPlayerData(event.player)
                val blockData = createBlockData(event.block)
                adapter.eventManager.onPlayerBreakBlockAfter(playerData, blockData)
            }
        }
    }
    
    @EventHandler(priority = EventPriority.HIGH)
    fun onBlockBreakBefore(event: BlockBreakEvent) {
        scheduleRegionTask(event.block.location) {
            val playerData = createPlayerData(event.player)
            val blockData = createBlockData(event.block)
            adapter.eventManager.onPlayerBreakBlockBefore(playerData, blockData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onAsyncPlayerChat(event: AsyncPlayerChatEvent) {
        if (!event.isCancelled) {
            // Chat events can be processed immediately as they're already async
            val playerData = createPlayerData(event.player)
            adapter.eventManager.onPlayerChat(playerData, event.message)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onWorldLoad(event: WorldLoadEvent) {
        scheduleGlobalTask {
            val worldData = createWorldData(event.world)
            adapter.eventManager.onWorldLoad(worldData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onWorldUnload(event: WorldUnloadEvent) {
        scheduleGlobalTask {
            val worldData = createWorldData(event.world)
            adapter.eventManager.onWorldUnload(worldData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onPlayerAdvancementDone(event: PlayerAdvancementDoneEvent) {
        scheduleEntityTask(event.player) {
            val playerData = createPlayerData(event.player)
            val advancementData = createAdvancementData(event.advancement)
            adapter.eventManager.onAdvancementComplete(playerData, advancementData)
        }
    }
    
    @EventHandler(priority = EventPriority.MONITOR)
    fun onServerLoad(event: ServerLoadEvent) {
        scheduleGlobalTask {
            when (event.type) {
                ServerLoadEvent.LoadType.STARTUP -> adapter.eventManager.onServerStarted()
                ServerLoadEvent.LoadType.RELOAD -> {
                    adapter.eventManager.onDataPackReloadStart()
                    adapter.eventManager.onDataPackReloadEnd(true)
                }
            }
        }
    }
    
    // Folia-specific scheduling methods
    
    /**
     * Schedule a task on the entity's scheduler (Folia-specific)
     */
    private fun scheduleEntityTask(entity: Entity, task: Runnable) {
        if (isFolia()) {
            try {
                entity.scheduler.run(this, { _ -> task.run() }, null)
            } catch (e: Exception) {
                super.getLogger().warning("Failed to schedule entity task: ${e.message}")
                // Fallback to immediate execution
                task.run()
            }
        } else {
            // Fallback for non-Folia
            Bukkit.getScheduler().runTask(this, task)
        }
    }
    
    /**
     * Schedule a task on the region scheduler (Folia-specific)
     */
    private fun scheduleRegionTask(location: org.bukkit.Location, task: Runnable) {
        if (isFolia()) {
            try {
                Bukkit.getRegionScheduler().run(this, location) { _ -> task.run() }
            } catch (e: Exception) {
                super.getLogger().warning("Failed to schedule region task: ${e.message}")
                // Fallback to immediate execution
                task.run()
            }
        } else {
            // Fallback for non-Folia
            Bukkit.getScheduler().runTask(this, task)
        }
    }
    
    /**
     * Schedule a task on the global region scheduler (Folia-specific)
     */
    private fun scheduleGlobalTask(task: Runnable) {
        if (isFolia()) {
            try {
                Bukkit.getGlobalRegionScheduler().run(this) { _ -> task.run() }
            } catch (e: Exception) {
                super.getLogger().warning("Failed to schedule global task: ${e.message}")
                // Fallback to immediate execution
                task.run()
            }
        } else {
            // Fallback for non-Folia
            Bukkit.getScheduler().runTask(this, task)
        }
    }
    
    // Command and server management
    
    private fun executeServerCommand(command: String): String {
        return try {
            // Commands should be executed on the main thread/global region
            var result = false
            if (isFolia()) {
                Bukkit.getGlobalRegionScheduler().run(this) { _ ->
                    result = Bukkit.dispatchCommand(Bukkit.getConsoleSender(), command)
                }
            } else {
                result = Bukkit.dispatchCommand(Bukkit.getConsoleSender(), command)
            }
            
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
            scheduleGlobalTask {
                Bukkit.broadcastMessage(message)
            }
            "Message broadcasted successfully"
        } catch (e: Exception) {
            super.getLogger().severe("Failed to broadcast message: ${e.message}")
            "Failed to broadcast message: ${e.message}"
        }
    }
    
    // Helper methods for data conversion (same as Paper)
    
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
    
}