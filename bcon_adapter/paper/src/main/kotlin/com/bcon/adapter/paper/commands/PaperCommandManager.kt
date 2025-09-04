package com.bcon.adapter.paper.commands

import com.bcon.adapter.core.commands.DynamicCommandManager
import com.bcon.adapter.core.commands.CommandDefinition
import com.bcon.adapter.core.commands.CommandSender
import com.bcon.adapter.core.commands.SenderType
import com.bcon.adapter.core.commands.ArgumentType
import org.bukkit.Bukkit
import org.bukkit.command.*
import org.bukkit.entity.Player
import org.bukkit.plugin.java.JavaPlugin
import java.util.logging.Logger

/**
 * Paper-specific command manager that integrates with Bukkit's command system
 * Handles registration and execution of dynamic commands in Paper/Bukkit
 */
class PaperCommandManager(
    private val plugin: JavaPlugin,
    private val coreManager: DynamicCommandManager
) {
    
    private val logger = Logger.getLogger(PaperCommandManager::class.java.simpleName)
    private val registeredCommands = mutableMapOf<String, DynamicBukkitCommand>()
    
    init {
        registerExistingCommands()
    }
    
    /**
     * Register all existing dynamic commands with Bukkit
     */
    private fun registerExistingCommands() {
        logger.info("Registering existing dynamic commands with Paper")
        
        coreManager.getAllCommands().forEach { (name, definition) ->
            registerBukkitCommand(definition)
        }
        
        logger.info("Registered ${coreManager.getAllCommands().size} dynamic commands with Paper")
    }
    
    /**
     * Register a single command with Bukkit
     */
    fun registerBukkitCommand(definition: CommandDefinition) {
        try {
            val command = DynamicBukkitCommand(definition, coreManager)
            registeredCommands[definition.name] = command
            
            // Register with the command map
            val commandMap = Bukkit.getCommandMap()
            commandMap.register(plugin.name.lowercase(), command)
            
            logger.info("Registered command: ${definition.name}")
            
        } catch (e: Exception) {
            logger.severe("Failed to register command '${definition.name}': ${e.message}")
        }
    }
    
    /**
     * Unregister a command from Bukkit
     */
    fun unregisterBukkitCommand(commandName: String) {
        try {
            val command = registeredCommands.remove(commandName)
            if (command != null) {
                command.unregister(Bukkit.getCommandMap())
                logger.info("Unregistered command: $commandName")
            }
        } catch (e: Exception) {
            logger.severe("Failed to unregister command '$commandName': ${e.message}")
        }
    }
    
    /**
     * Handle new command registration from the core manager
     */
    fun onCommandRegistered(definition: CommandDefinition) {
        registerBukkitCommand(definition)
    }
    
    /**
     * Handle command removal from the core manager
     */
    fun onCommandUnregistered(commandName: String) {
        unregisterBukkitCommand(commandName)
    }
    
    /**
     * Handle command updates from the core manager
     */
    fun onCommandUpdated(definition: CommandDefinition) {
        unregisterBukkitCommand(definition.name)
        registerBukkitCommand(definition)
    }
    
    /**
     * Shutdown the command manager
     */
    fun shutdown() {
        logger.info("Shutting down Paper command manager")
        
        registeredCommands.keys.toList().forEach { commandName ->
            unregisterBukkitCommand(commandName)
        }
        
        registeredCommands.clear()
        logger.info("Paper command manager shutdown complete")
    }
    
    /**
     * Get command registration status
     */
    fun getRegisteredCommands(): Map<String, DynamicBukkitCommand> = registeredCommands.toMap()
    
    /**
     * Check if a command is registered
     */
    fun isCommandRegistered(commandName: String): Boolean = registeredCommands.containsKey(commandName)
}

/**
 * Bukkit command wrapper for dynamic commands
 */
class DynamicBukkitCommand(
    private val definition: CommandDefinition,
    private val coreManager: DynamicCommandManager
) : Command(definition.name, definition.description, definition.usage, definition.aliases) {
    
    private val logger = Logger.getLogger("${DynamicBukkitCommand::class.java.simpleName}-${definition.name}")
    
    override fun execute(sender: org.bukkit.command.CommandSender, commandLabel: String, args: Array<out String>): Boolean {
        try {
            val bconSender = convertSender(sender)
            
            // Convert args array to argument map (simplified approach)
            val argMap = mutableMapOf<String, Any>()
            args.forEachIndexed { index, arg ->
                argMap["arg_$index"] = arg
            }
            
            val success = coreManager.executeCommand(definition.name, null, argMap, bconSender)
            
            if (!success) {
                sender.sendMessage("§cCommand execution failed.")
            }
            
            return success
            
        } catch (e: Exception) {
            logger.severe("Error executing command '${definition.name}': ${e.message}")
            sender.sendMessage("§cAn error occurred while executing the command.")
            return false
        }
    }
    
    override fun tabComplete(sender: org.bukkit.command.CommandSender, alias: String, args: Array<out String>): List<String> {
        // Tab completion not implemented in core yet - return empty list
        return emptyList()
    }
    
    /**
     * Convert Bukkit command sender to Bcon command sender
     */
    private fun convertSender(bukkitSender: org.bukkit.command.CommandSender): CommandSender {
        return when (bukkitSender) {
            is Player -> CommandSender(
                name = bukkitSender.name,
                type = SenderType.PLAYER,
                uuid = bukkitSender.uniqueId.toString()
            )
            is ConsoleCommandSender -> CommandSender(
                name = "Console",
                type = SenderType.CONSOLE,
                uuid = "console"
            )
            is BlockCommandSender -> CommandSender(
                name = "CommandBlock",
                type = SenderType.COMMAND_BLOCK,
                uuid = "command-block"
            )
            else -> CommandSender(
                name = bukkitSender.name,
                type = SenderType.CONSOLE, // Default to CONSOLE since OTHER doesn't exist
                uuid = "unknown"
            )
        }
    }
}