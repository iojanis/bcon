package com.bcon.adapter.fabric

import com.bcon.adapter.core.BconAdapter
import com.bcon.adapter.core.events.*
import com.bcon.adapter.fabric.commands.FabricCommandManager
import com.bcon.adapter.fabric.integration.FabricBlueMapIntegration
import com.bcon.adapter.fabric.logging.FabricBconLogger
import com.google.gson.JsonObject
import net.fabricmc.api.DedicatedServerModInitializer
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback
import net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents
import net.fabricmc.fabric.api.entity.event.v1.ServerPlayerEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerWorldEvents
import net.fabricmc.fabric.api.event.player.*
import net.fabricmc.fabric.api.message.v1.ServerMessageEvents
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents
import net.minecraft.advancement.Advancement
import net.minecraft.advancement.AdvancementProgress
import net.minecraft.block.BlockState
import net.minecraft.entity.Entity
import net.minecraft.entity.LivingEntity
import net.minecraft.entity.damage.DamageSource
import net.minecraft.entity.player.PlayerEntity
import net.minecraft.entity.projectile.ProjectileEntity
import net.minecraft.server.MinecraftServer
import net.minecraft.server.command.ServerCommandSource
import net.minecraft.server.network.ServerPlayerEntity
import net.minecraft.server.world.ServerWorld
import net.minecraft.text.Text
import net.minecraft.util.math.BlockPos
import net.minecraft.registry.Registries
import net.minecraft.world.World

/**
 * Fabric implementation of the Bcon adapter
 * Handles Fabric-specific event registration and server integration
 */
class FabricBconAdapter : BconAdapter(), DedicatedServerModInitializer {
    
    override val logger = FabricBconLogger("Fabric")
    private var server: MinecraftServer? = null
    private var fabricCommandManager: FabricCommandManager? = null
    private var fabricBlueMapIntegration: FabricBlueMapIntegration? = null
    
    override fun onInitializeServer() {
        logger.info("Initializing Fabric Bcon Adapter")
        initialize()
    }
    
    override fun onInitialize() {
        logger.info("Setting up Fabric-specific components")
        
        // Initialize Fabric-specific command manager
        fabricCommandManager = FabricCommandManager(this, commandManager)
        
        // Initialize Fabric-specific BlueMap integration if available
        try {
            if (config.enableBlueMap) {
                fabricBlueMapIntegration = FabricBlueMapIntegration(this)
                logger.info("Fabric BlueMap integration enabled")
            }
        } catch (e: Exception) {
            logger.info("BlueMap not available for Fabric, integration disabled")
        }
    }
    
    override fun onShutdown() {
        logger.info("Shutting down Fabric Bcon Adapter")
        fabricBlueMapIntegration?.shutdown()
        fabricCommandManager?.shutdown()
    }
    
    override fun registerEvents() {
        logger.info("Registering Fabric events")
        
        // Server lifecycle events
        ServerLifecycleEvents.SERVER_STARTING.register { server ->
            this.server = server
            eventManager.onServerStarting()
        }
        
        ServerLifecycleEvents.SERVER_STARTED.register { server ->
            eventManager.onServerStarted()
        }
        
        ServerLifecycleEvents.SERVER_STOPPING.register { server ->
            eventManager.onServerStopping()
        }
        
        ServerLifecycleEvents.SERVER_STOPPED.register { server ->
            eventManager.onServerStopped()
            this.server = null
        }
        
        // World events
        ServerWorldEvents.LOAD.register { server, world ->
            val worldData = createWorldData(world)
            eventManager.onWorldLoad(worldData)
        }
        
        ServerWorldEvents.UNLOAD.register { server, world ->
            val worldData = createWorldData(world)
            eventManager.onWorldUnload(worldData)
        }
        
        // Player connection events
        ServerPlayConnectionEvents.JOIN.register { handler, sender, server ->
            val player = handler.player
            val playerData = createPlayerData(player as ServerPlayerEntity)
            eventManager.onPlayerJoined(playerData)
        }
        
        ServerPlayConnectionEvents.DISCONNECT.register { handler, server ->
            val player = handler.player
            val playerData = createPlayerData(player as ServerPlayerEntity)
            eventManager.onPlayerLeft(playerData)
        }
        
        ServerPlayConnectionEvents.INIT.register { handler, server ->
            val player = handler.player
            val playerData = createPlayerData(player as ServerPlayerEntity)
            eventManager.onPlayerConnectionInit(playerData)
        }
        
        // Player respawn events
        ServerPlayerEvents.AFTER_RESPAWN.register { oldPlayer, newPlayer, alive ->
            val oldPlayerData = createPlayerData(oldPlayer)
            val newPlayerData = createPlayerData(newPlayer)
            eventManager.onPlayerRespawned(oldPlayerData, newPlayerData, alive)
        }
        
        // Entity death events
        ServerLivingEntityEvents.AFTER_DEATH.register { entity, damageSource ->
            if (entity is ServerPlayerEntity) {
                val playerData = createPlayerData(entity)
                val deathMessage = entity.getDamageTracker().getDeathMessage().string
                val attacker = damageSource.attacker?.let { createEntityData(it) }
                eventManager.onPlayerDeath(playerData, deathMessage, attacker)
            } else {
                val killedEntity = createEntityData(entity)
                val killer = damageSource.attacker?.let { createEntityData(it) }
                val deathMessage = "Entity death" // Fabric doesn't provide death messages for non-players
                eventManager.onEntityDeath(killer, killedEntity, deathMessage)
            }
        }
        
        // Block break events
        PlayerBlockBreakEvents.BEFORE.register { world, player, pos, state, blockEntity ->
            val playerData = createPlayerData(player as ServerPlayerEntity)
            val blockData = createBlockData(state, pos, world)
            eventManager.onPlayerBreakBlockBefore(playerData, blockData)
            true // Allow the event to continue
        }
        
        PlayerBlockBreakEvents.AFTER.register { world, player, pos, state, blockEntity ->
            val playerData = createPlayerData(player as ServerPlayerEntity)
            val blockData = createBlockData(state, pos, world)
            eventManager.onPlayerBreakBlockAfter(playerData, blockData)
        }
        
        // Chat events
        ServerMessageEvents.CHAT_MESSAGE.register { message, sender, params ->
            val playerData = createPlayerData(sender)
            eventManager.onPlayerChat(playerData, message.content.string)
        }
        
        // Command registration
        CommandRegistrationCallback.EVENT.register { dispatcher, registryAccess, environment ->
            fabricCommandManager?.registerFabricCommands(dispatcher)
        }
        
        // Advancement events (custom implementation needed)
        registerAdvancementEvents()
        
        logger.info("Fabric events registered successfully")
    }
    
    override fun executeCommand(command: String): String {
        val server = this.server ?: return "Server not available"
        
        return try {
            val commandSource = server.commandSource
            val commandManager = server.commandManager
            val result = commandManager.executeWithPrefix(commandSource, command)
            "Command executed successfully (result: $result)"
        } catch (e: Exception) {
            logger.severe("Failed to execute command '$command': ${e.message}")
            "Failed to execute command: ${e.message}"
        }
    }
    
    override fun broadcastMessage(message: String): String {
        val server = this.server ?: return "Server not available"
        
        return try {
            val text = Text.literal(message)
            server.playerManager.broadcast(text, false)
            "Message broadcasted successfully"
        } catch (e: Exception) {
            logger.severe("Failed to broadcast message: ${e.message}")
            "Failed to broadcast message: ${e.message}"
        }
    }
    
    override fun getServerInstance(): Any? = server
    
    override fun isServerRunning(): Boolean = server?.isRunning == true
    
    override fun getServerInfo(): JsonObject {
        val server = this.server
        return JsonObject().apply {
            addProperty("platform", "Fabric")
            addProperty("running", server?.isRunning ?: false)
            addProperty("version", server?.version ?: "Unknown")
            addProperty("playerCount", server?.currentPlayerCount ?: 0)
            addProperty("maxPlayers", server?.maxPlayerCount ?: 0)
            addProperty("worldCount", server?.worlds?.count() ?: 0)
        }
    }
    
    // Helper methods for data conversion
    
    private fun createPlayerData(player: ServerPlayerEntity): PlayerData {
        return PlayerData(
            uuid = player.uuidAsString,
            name = player.gameProfile.name,
            location = createLocation(player.pos, player.world),
            health = player.health.toDouble(),
            maxHealth = player.maxHealth.toDouble(),
            level = player.experienceLevel,
            gameMode = player.interactionManager.gameMode.name
        )
    }
    
    private fun createEntityData(entity: Entity): EntityData {
        return EntityData(
            uuid = entity.uuidAsString,
            type = Registries.ENTITY_TYPE.getId(entity.type).toString(),
            location = createLocation(entity.pos, entity.world),
            name = if (entity is LivingEntity) entity.displayName?.string else null
        )
    }
    
    private fun createWorldData(world: ServerWorld): WorldData {
        return WorldData(
            name = world.registryKey.value.toString(),
            dimensionKey = world.registryKey.value.toString(),
            time = world.timeOfDay,
            difficulty = world.difficulty.name,
            weather = if (world.isRaining) "RAIN" else "CLEAR",
            thundering = world.isThundering
        )
    }
    
    private fun createBlockData(blockState: BlockState, pos: BlockPos, world: World): BlockData {
        return BlockData(
            type = Registries.BLOCK.getId(blockState.block).toString(),
            location = Location(
                x = pos.x.toDouble(),
                y = pos.y.toDouble(), 
                z = pos.z.toDouble(),
                dimension = world.registryKey.value.toString()
            ),
            data = blockState.toString()
        )
    }
    
    private fun createLocation(pos: net.minecraft.util.math.Vec3d, world: World): Location {
        return Location(
            x = pos.x,
            y = pos.y,
            z = pos.z,
            dimension = world.registryKey.value.toString()
        )
    }
    
    private fun createAdvancementData(advancement: Advancement): AdvancementData {
        return AdvancementData(
            id = "advancement_${System.currentTimeMillis()}", // Simplified ID as advancement.id may not be accessible
            title = "Advancement", // Simplified title
            description = "Player completed an advancement" // Simplified description
        )
    }
    
    private fun registerAdvancementEvents() {
        // Custom advancement event registration
        // This requires more complex implementation as Fabric doesn't provide direct advancement events
        ServerTickEvents.END_SERVER_TICK.register { server ->
            // Check for advancement completion periodically
            // This is a simplified approach - a more sophisticated implementation would track changes
            for (player in server.playerManager.playerList) {
                checkPlayerAdvancements(player)
            }
        }
    }
    
    private val lastAdvancementCheck = mutableMapOf<String, Long>()
    
    private fun checkPlayerAdvancements(player: ServerPlayerEntity) {
        // Simplified advancement tracking - temporarily disabled due to complex API interactions
        // Will implement when needed and Fabric API is more stable for advancement tracking
        logger.fine("Advancement checking disabled for Fabric (API complexity)")
    }
    
    override fun handleStrictModeFailure() {
        logger.severe("Bcon connection failed in strict mode - shutting down server")
        
        // Schedule server shutdown on the main thread
        getServerInstance()?.let { server ->
            if (server is MinecraftServer) {
                server.execute {
                    logger.severe("Executing emergency server shutdown due to Bcon connection failure")
                    server.stop(false)
                }
            }
        }
    }
    
}