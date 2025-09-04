package com.bcon.adapter.fabric.commands

import com.bcon.adapter.core.commands.DynamicCommandManager
import com.bcon.adapter.core.commands.CommandDefinition
import com.bcon.adapter.core.commands.CommandSender
import com.bcon.adapter.core.commands.SenderType
import com.bcon.adapter.fabric.FabricBconAdapter
import com.mojang.brigadier.CommandDispatcher
import com.mojang.brigadier.arguments.*
import com.mojang.brigadier.builder.LiteralArgumentBuilder
import com.mojang.brigadier.builder.RequiredArgumentBuilder
import com.mojang.brigadier.context.CommandContext
import com.mojang.brigadier.exceptions.CommandSyntaxException
import net.minecraft.command.argument.*
import net.minecraft.server.command.CommandManager
import net.minecraft.server.command.ServerCommandSource
import net.minecraft.text.Text
import com.bcon.adapter.core.logging.BconLogger

/**
 * Fabric-specific command manager that integrates with Brigadier
 * Handles registration and execution of dynamic commands in Fabric
 */
class FabricCommandManager(
    private val adapter: FabricBconAdapter,
    private val coreManager: DynamicCommandManager
) {
    
    private val logger: BconLogger = adapter.logger
    
    /**
     * Register all dynamic commands with Fabric's Brigadier system
     */
    fun registerFabricCommands(dispatcher: CommandDispatcher<ServerCommandSource>) {
        logger.info("Registering dynamic commands with Fabric Brigadier")
        
        // Register built-in /bcon command
        registerBconCommand(dispatcher)
        
        coreManager.getAllCommands().forEach { (name, definition) ->
            registerBrigadierCommand(dispatcher, definition)
        }
        
        logger.info("Registered ${coreManager.getAllCommands().size} dynamic commands with Brigadier")
    }
    
    /**
     * Register the built-in /bcon command for configuration management
     */
    private fun registerBconCommand(dispatcher: CommandDispatcher<ServerCommandSource>) {
        dispatcher.register(
            CommandManager.literal("bcon")
                .requires { source -> source.hasPermissionLevel(4) } // Operator permission
                .then(
                    CommandManager.literal("status")
                        .executes { context -> executeBconCommand(context, "status", null) }
                )
                .then(
                    CommandManager.literal("token")
                        .then(
                            CommandManager.argument("value", StringArgumentType.greedyString())
                                .executes { context -> 
                                    val token = StringArgumentType.getString(context, "value")
                                    executeBconCommand(context, "token", token) 
                                }
                        )
                )
                .then(
                    CommandManager.literal("url")
                        .then(
                            CommandManager.argument("value", StringArgumentType.greedyString())
                                .executes { context -> 
                                    val url = StringArgumentType.getString(context, "value")
                                    executeBconCommand(context, "url", url) 
                                }
                        )
                )
                .then(
                    CommandManager.literal("id")
                        .then(
                            CommandManager.argument("value", StringArgumentType.greedyString())
                                .executes { context -> 
                                    val id = StringArgumentType.getString(context, "value")
                                    executeBconCommand(context, "id", id) 
                                }
                        )
                )
                .then(
                    CommandManager.literal("name")
                        .then(
                            CommandManager.argument("value", StringArgumentType.greedyString())
                                .executes { context -> 
                                    val name = StringArgumentType.getString(context, "value")
                                    executeBconCommand(context, "name", name) 
                                }
                        )
                )
                .then(
                    CommandManager.literal("strict")
                        .then(
                            CommandManager.argument("value", BoolArgumentType.bool())
                                .executes { context -> 
                                    val strict = BoolArgumentType.getBool(context, "value").toString()
                                    executeBconCommand(context, "strict", strict) 
                                }
                        )
                )
                .then(
                    CommandManager.literal("reconnect")
                        .executes { context -> executeBconCommand(context, "reconnect", null) }
                )
                .then(
                    CommandManager.literal("reload")
                        .executes { context -> executeBconCommand(context, "reload", null) }
                )
                .executes { context -> 
                    // Show usage when no subcommand is provided
                    executeBconCommand(context, "help", null)
                }
        )
    }
    
    /**
     * Execute a /bcon command
     */
    private fun executeBconCommand(context: CommandContext<ServerCommandSource>, action: String, value: String?): Int {
        return try {
            val data = com.google.gson.JsonObject().apply {
                addProperty("action", action)
                value?.let { addProperty("value", it) }
            }
            
            val result = if (action == "help") {
                "Bcon Commands:\n" +
                "- /bcon status - Show connection status\n" +
                "- /bcon token <jwt> - Update JWT token\n" +
                "- /bcon url <ws://host:port> - Update server URL\n" +
                "- /bcon id <id> - Update server ID\n" +
                "- /bcon name <name> - Update server name\n" +
                "- /bcon strict <true|false> - Toggle strict mode\n" +
                "- /bcon reconnect - Force reconnection\n" +
                "- /bcon reload - Reload configuration"
            } else {
                adapter.handleBconConfigCommand(data)
            }
            
            // Send result to command source
            context.source.sendFeedback({ Text.literal(result) }, true)
            1
        } catch (e: Exception) {
            logger.severe("Error executing /bcon command: ${e.message}")
            context.source.sendError(Text.literal("Error executing command: ${e.message}"))
            0
        }
    }
    
    /**
     * Register a single command with Brigadier
     */
    private fun registerBrigadierCommand(
        dispatcher: CommandDispatcher<ServerCommandSource>,
        definition: CommandDefinition
    ) {
        try {
            val baseCommand = CommandManager.literal(definition.name)
            
            if (definition.subcommands.isNotEmpty()) {
                // Command has subcommands
                definition.subcommands.forEach { subcommand ->
                    val subcommandNode = CommandManager.literal(subcommand.name)
                    
                    // Add arguments to subcommand
                    val finalNode = addArgumentsToCommand(subcommandNode, subcommand.arguments)
                    finalNode.executes { context ->
                        executeCommand(context, definition.name, subcommand.name)
                    }
                    
                    baseCommand.then(finalNode)
                }
            } else {
                // Command has direct arguments
                val finalCommand = addArgumentsToCommand(baseCommand, definition.arguments)
                finalCommand.executes { context ->
                    executeCommand(context, definition.name, null)
                }
                
                dispatcher.register(finalCommand)
                return
            }
            
            dispatcher.register(baseCommand)
            logger.fine("Registered Brigadier command: ${definition.name}")
            
        } catch (e: Exception) {
            logger.severe("Failed to register Brigadier command '${definition.name}': ${e.message}")
        }
    }
    
    /**
     * Add arguments to a command builder
     */
    private fun addArgumentsToCommand(
        command: LiteralArgumentBuilder<ServerCommandSource>,
        arguments: List<com.bcon.adapter.core.commands.ArgumentDefinition>
    ): LiteralArgumentBuilder<ServerCommandSource> {
        var currentCommand = command
        
        arguments.forEach { arg ->
            val argumentBuilder = when (arg.type) {
                com.bcon.adapter.core.commands.ArgumentType.STRING -> {
                    if (arg.suggestions.isNotEmpty()) {
                        CommandManager.argument(arg.name, StringArgumentType.word())
                    } else {
                        CommandManager.argument(arg.name, StringArgumentType.greedyString())
                    }
                }
                com.bcon.adapter.core.commands.ArgumentType.INTEGER -> {
                    val intArg = IntegerArgumentType.integer(
                        arg.min?.toInt() ?: Int.MIN_VALUE,
                        arg.max?.toInt() ?: Int.MAX_VALUE
                    )
                    CommandManager.argument(arg.name, intArg)
                }
                com.bcon.adapter.core.commands.ArgumentType.DOUBLE -> {
                    val doubleArg = DoubleArgumentType.doubleArg(
                        arg.min ?: Double.MIN_VALUE,
                        arg.max ?: Double.MAX_VALUE
                    )
                    CommandManager.argument(arg.name, doubleArg)
                }
                com.bcon.adapter.core.commands.ArgumentType.BOOLEAN -> {
                    CommandManager.argument(arg.name, BoolArgumentType.bool())
                }
                com.bcon.adapter.core.commands.ArgumentType.PLAYER -> {
                    CommandManager.argument(arg.name, EntityArgumentType.player())
                }
                com.bcon.adapter.core.commands.ArgumentType.WORLD -> {
                    CommandManager.argument(arg.name, DimensionArgumentType.dimension())
                }
                com.bcon.adapter.core.commands.ArgumentType.ITEM -> {
                    CommandManager.argument(arg.name, ItemStackArgumentType.itemStack(null))
                }
                com.bcon.adapter.core.commands.ArgumentType.LOCATION -> {
                    CommandManager.argument(arg.name, Vec3ArgumentType.vec3())
                }
                com.bcon.adapter.core.commands.ArgumentType.ENUM -> {
                    CommandManager.argument(arg.name, StringArgumentType.word())
                }
            }
            
            // Add suggestions if available
            if (arg.suggestions.isNotEmpty()) {
                argumentBuilder.suggests { context, builder ->
                    arg.suggestions.forEach { suggestion ->
                        builder.suggest(suggestion)
                    }
                    builder.buildFuture()
                }
            }
            
            currentCommand = currentCommand.then(argumentBuilder)
        }
        
        return currentCommand
    }
    
    /**
     * Execute a dynamic command through the core manager
     */
    private fun executeCommand(
        context: CommandContext<ServerCommandSource>,
        commandName: String,
        subcommandName: String?
    ): Int {
        try {
            val source = context.source
            val sender = createCommandSender(source)
            val arguments = extractArguments(context)
            
            val success = coreManager.executeCommand(commandName, subcommandName, arguments, sender)
            
            if (!success) {
                source.sendError(Text.literal("Failed to execute command: $commandName"))
                return 0
            }
            
            return 1
            
        } catch (e: CommandSyntaxException) {
            logger.warning("Command syntax error: ${e.message}")
            context.source.sendError(Text.literal("Command syntax error: ${e.message}"))
            return 0
        } catch (e: Exception) {
            logger.severe("Error executing command '$commandName': ${e.message}")
            context.source.sendError(Text.literal("Command execution error: ${e.message}"))
            return 0
        }
    }
    
    /**
     * Create a CommandSender from Minecraft's command source
     */
    private fun createCommandSender(source: ServerCommandSource): CommandSender {
        val entity = source.entity
        return when {
            entity?.isPlayer == true -> CommandSender(
                name = entity.displayName?.string ?: "Unknown Player",
                type = SenderType.PLAYER,
                uuid = entity.uuidAsString
            )
            source.name == "Server" -> CommandSender(
                name = "Console",
                type = SenderType.CONSOLE
            )
            else -> CommandSender(
                name = source.name,
                type = SenderType.COMMAND_BLOCK
            )
        }
    }
    
    /**
     * Extract arguments from Brigadier command context
     */
    private fun extractArguments(context: CommandContext<ServerCommandSource>): Map<String, Any> {
        val arguments = mutableMapOf<String, Any>()
        
        // Use empty arguments for now due to private field access limitation
        
        return arguments
    }
    
    /**
     * Refresh commands by re-registering them
     * This should be called when commands are added/removed/updated
     */
    fun refreshCommands() {
        val serverInstance = adapter.getServerInstance()
        if (serverInstance != null) {
            try {
                val server = serverInstance as net.minecraft.server.MinecraftServer
                // Clear and re-register all commands
                val commandManager = server.commandManager
                val dispatcher = commandManager.dispatcher
                
                // Note: Brigadier doesn't support command removal easily
                // In a real implementation, you might need to recreate the dispatcher
                logger.info("Command refresh requested - server restart may be required for full effect")
                
            } catch (e: Exception) {
                logger.severe("Failed to refresh commands: ${e.message}")
            }
        }
    }
    
    /**
     * Shutdown the command manager
     */
    fun shutdown() {
        logger.info("Shutting down Fabric command manager")
    }
}