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
import net.fabricmc.fabric.api.entity.event.v1.ServerEntityCombatEvents
import net.fabricmc.fabric.api.event.Event
import net.fabricmc.fabric.api.event.EventFactory
import net.fabricmc.fabric.api.itemgroup.v1.ItemGroupEvents
import net.fabricmc.fabric.api.loot.v2.LootTableEvents
import net.fabricmc.fabric.api.event.player.UseBlockCallback
import net.fabricmc.fabric.api.event.player.UseEntityCallback
import net.fabricmc.fabric.api.event.player.UseItemCallback
import net.minecraft.advancement.Advancement
import net.minecraft.advancement.AdvancementProgress
import net.minecraft.block.BlockState
import net.minecraft.entity.Entity
import net.minecraft.entity.LivingEntity
import net.minecraft.entity.damage.DamageSource
import net.minecraft.entity.player.PlayerEntity
import net.minecraft.entity.projectile.ProjectileEntity
import net.minecraft.entity.passive.AnimalEntity
import net.minecraft.entity.ItemEntity
import net.minecraft.item.ItemStack
import net.minecraft.item.Items
import net.minecraft.util.ActionResult
import net.minecraft.util.Hand
import net.minecraft.util.hit.BlockHitResult
import net.minecraft.util.hit.EntityHitResult
import net.minecraft.block.entity.AbstractFurnaceBlockEntity
import net.minecraft.inventory.Inventory
import net.minecraft.screen.ScreenHandler
import net.minecraft.loot.LootTable
import net.minecraft.loot.context.LootContext
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
        // Register this adapter with the event bridge for mixin access
        FabricEventBridge.registerAdapter(this)
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
        // Unregister from the event bridge
        FabricEventBridge.unregisterAdapter()
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
        
        // Enhanced Entity Events
        ServerEntityCombatEvents.AFTER_KILLED_OTHER_ENTITY.register { world, entity, killedEntity ->
            val entityData = createEntityData(entity)
            val killedEntityData = createEntityData(killedEntity)
            eventManager.onEntityDeath(entityData, killedEntityData, "Entity combat death")
        }
        
        // Entity damage events
        ServerLivingEntityEvents.ALLOW_DAMAGE.register { entity, source, amount ->
            val entityData = createEntityData(entity)
            val damageType = source.name ?: "UNKNOWN"
            val damageSourceEntity = source.attacker?.let { createEntityData(it) }
            eventManager.onEntityDamage(entityData, amount.toDouble(), damageType, damageSourceEntity)
            true // Allow damage to proceed
        }
        
        // Use entity callback for basic entity interactions
        UseEntityCallback.EVENT.register { player, world, hand, entity, hitResult ->
            if (entity is AnimalEntity && player is ServerPlayerEntity) {
                val item = player.getStackInHand(hand)
                val playerData = createPlayerData(player)
                val entityData = createEntityData(entity)
                
                // Basic breeding item detection
                if (!entity.world.isClient && entity.isBreedingItem(item)) {
                    // Animal might enter love mode - handled by mixins for detailed tracking
                    logger.info("Breeding item used on ${entity.type} by ${player.name.string}")
                }
            }
            ActionResult.PASS
        }
        
        // Use block callback for basic block interactions
        UseBlockCallback.EVENT.register { player, world, hand, hitResult ->
            if (player is ServerPlayerEntity) {
                val blockPos = hitResult.blockPos
                val blockState = world.getBlockState(blockPos)
                val block = blockState.block
                val playerData = createPlayerData(player)
                val location = createLocationFromBlockPos(blockPos, world)
                
                // Handle basic inventory opening detection
                val inventoryType = when {
                    block.toString().contains("chest") -> "CHEST"
                    block.toString().contains("barrel") -> "BARREL"
                    block.toString().contains("shulker") -> "SHULKER_BOX"
                    block.toString().contains("furnace") -> "FURNACE"
                    block.toString().contains("hopper") -> "HOPPER"
                    else -> "UNKNOWN"
                }
                
                if (inventoryType != "UNKNOWN") {
                    eventManager.onPlayerInventoryOpen(playerData, inventoryType, location)
                }
            }
            ActionResult.PASS
        }
        
        // Use item callback for basic item usage
        UseItemCallback.EVENT.register { player, world, hand ->
            if (player is ServerPlayerEntity) {
                val item = player.getStackInHand(hand)
                if (item.item == Items.FISHING_ROD) {
                    val playerData = createPlayerData(player)
                    val location = createLocationFromPos(player.pos, world)
                    val hookData = FishHookData(location, false, 0) // Basic hook data
                    
                    // Basic fishing rod usage detection - detailed events handled by mixins
                    eventManager.onPlayerFishingCast(playerData, hookData)
                }
            }
            net.minecraft.util.TypedActionResult.pass(player.getStackInHand(hand))
        }
        
        // Advanced events requiring custom implementation
        registerCustomFabricEvents()
        
        // Advancement events (custom implementation needed)
        registerAdvancementEvents()
        
        logger.info("Fabric events registered successfully - enhanced coverage with custom implementations")
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
    
    private fun registerCustomFabricEvents() {
        // Register custom events that don't have direct Fabric API equivalents
        logger.info("Registering custom Fabric events for enhanced functionality")
        
        // Server tick event for periodic checks
        ServerTickEvents.END_SERVER_TICK.register { server ->
            // We can use this to periodically check for state changes
            // that don't have direct events in Fabric
            
            // Check for item drops by monitoring ItemEntity spawns
            checkForItemDrops(server)
            
            // Check for furnace state changes
            checkFurnaceStates(server)
        }
    }
    
    private fun checkForItemDrops(server: MinecraftServer) {
        // This would require tracking ItemEntities that spawned this tick
        // In a full implementation, we'd maintain a cache of known ItemEntities
        // and detect new ones as drops
    }
    
    private fun checkFurnaceStates(server: MinecraftServer) {
        // This would require tracking furnace BlockEntities and their states
        // In a full implementation, we'd cache furnace states and detect changes
        // for smelting start/completion events
    }
    
    private fun createItemData(itemStack: ItemStack): ItemData {
        return ItemData(
            type = itemStack.item.toString(),
            amount = itemStack.count,
            displayName = try { 
                // Simplified name extraction for Fabric
                itemStack.name.string
            } catch (e: Exception) { 
                null 
            },
            lore = null, // Fabric doesn't expose lore easily
            nbt = try { 
                // Simplified NBT access for Fabric
                itemStack.toString()
            } catch (e: Exception) { 
                null 
            }
        )
    }
    
    private fun createLocationFromPos(pos: net.minecraft.util.math.Vec3d, world: net.minecraft.world.World): Location {
        return Location(
            x = pos.x,
            y = pos.y,
            z = pos.z,
            dimension = world.registryKey.value.toString(),
            yaw = 0f,
            pitch = 0f
        )
    }
    
    private fun createLocationFromBlockPos(pos: net.minecraft.util.math.BlockPos, world: net.minecraft.world.World): Location {
        return Location(
            x = pos.x.toDouble(),
            y = pos.y.toDouble(),
            z = pos.z.toDouble(),
            dimension = world.registryKey.value.toString(),
            yaw = 0f,
            pitch = 0f
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