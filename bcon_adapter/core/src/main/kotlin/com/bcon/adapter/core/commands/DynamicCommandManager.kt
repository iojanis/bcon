package com.bcon.adapter.core.commands

import com.bcon.adapter.core.BconAdapter
import com.google.gson.*
import java.io.File
import java.io.FileReader
import java.io.FileWriter
import java.util.concurrent.ConcurrentHashMap

/**
 * Improved dynamic command management system
 * Supports complex argument types, subcommands, and better error handling
 */
class DynamicCommandManager(private val adapter: BconAdapter) {
    
    private val logger = adapter.logger
    private val gson = GsonBuilder().setPrettyPrinting().create()
    private val commandsFile = File("plugins/bcon/commands.json")
    
    private val registeredCommands = ConcurrentHashMap<String, CommandDefinition>()
    private val commandExecutions = ConcurrentHashMap<String, MutableList<CommandExecution>>()
    
    init {
        loadCommands()
    }
    
    /**
     * Register a new dynamic command
     */
    fun registerCommand(commandData: JsonObject) {
        try {
            val definition = parseCommandDefinition(commandData)
            registeredCommands[definition.name] = definition
            
            logger.info("Registered command: ${definition.name}")
            
            // Notify platform to register the command
            registerPlatformCommand(definition)
            
            // Save to file
            saveCommands()
            
        } catch (e: Exception) {
            logger.severe("Failed to register command: ${e.message}")
            throw e
        }
    }
    
    /**
     * Unregister a command
     */
    fun unregisterCommand(commandName: String) {
        registeredCommands.remove(commandName)
        commandExecutions.remove(commandName)
        
        logger.info("Unregistered command: $commandName")
        saveCommands()
    }
    
    /**
     * Clear all registered commands
     */
    fun clearCommands() {
        registeredCommands.clear()
        commandExecutions.clear()
        
        logger.info("Cleared all registered commands")
        saveCommands()
    }
    
    /**
     * Execute a dynamic command
     */
    fun executeCommand(
        commandName: String,
        subcommand: String? = null,
        arguments: Map<String, Any>,
        sender: CommandSender
    ): Boolean {
        val definition = registeredCommands[commandName] ?: return false
        
        try {
            // Create execution record
            val execution = CommandExecution(
                command = commandName,
                subcommand = subcommand,
                arguments = arguments,
                sender = sender,
                timestamp = System.currentTimeMillis()
            )
            
            // Store execution for history
            commandExecutions.computeIfAbsent(commandName) { mutableListOf() }.add(execution)
            
            // Send execution event to Bcon server
            sendCommandExecutionEvent(execution)
            
            // Handle built-in responses
            handleBuiltinResponse(definition, execution)
            
            return true
            
        } catch (e: Exception) {
            logger.severe("Error executing command '$commandName': ${e.message}")
            return false
        }
    }
    
    /**
     * Get command definition
     */
    fun getCommandDefinition(name: String): CommandDefinition? {
        return registeredCommands[name]
    }
    
    /**
     * Get all registered commands
     */
    fun getAllCommands(): Map<String, CommandDefinition> {
        return registeredCommands.toMap()
    }
    
    /**
     * Get command execution history
     */
    fun getCommandHistory(commandName: String): List<CommandExecution> {
        return commandExecutions[commandName]?.toList() ?: emptyList()
    }
    
    /**
     * Shutdown and cleanup
     */
    fun shutdown() {
        saveCommands()
        registeredCommands.clear()
        commandExecutions.clear()
    }
    
    // Private methods
    
    /**
     * Load commands from file
     */
    private fun loadCommands() {
        try {
            commandsFile.parentFile?.mkdirs()
            
            if (!commandsFile.exists()) {
                logger.info("Commands file not found, creating new one")
                saveCommands()
                return
            }
            
            FileReader(commandsFile).use { reader ->
                val jsonArray = gson.fromJson(reader, JsonArray::class.java)
                
                for (element in jsonArray) {
                    try {
                        val commandData = element.asJsonObject
                        val definition = parseCommandDefinition(commandData)
                        registeredCommands[definition.name] = definition
                    } catch (e: Exception) {
                        logger.warning("Failed to load command: ${e.message}")
                    }
                }
            }
            
            logger.info("Loaded ${registeredCommands.size} commands from file")
            
        } catch (e: Exception) {
            logger.severe("Failed to load commands: ${e.message}")
            
            // Create backup of problematic file
            try {
                val backupFile = File(commandsFile.parent, "commands.json.backup")
                if (commandsFile.exists()) {
                    commandsFile.copyTo(backupFile, overwrite = true)
                    logger.info("Created backup at ${backupFile.absolutePath}")
                }
            } catch (backupError: Exception) {
                logger.warning("Failed to create backup: ${backupError.message}")
            }
            
            registeredCommands.clear()
            saveCommands()
        }
    }
    
    /**
     * Save commands to file
     */
    private fun saveCommands() {
        try {
            val jsonArray = JsonArray()
            
            for (definition in registeredCommands.values) {
                jsonArray.add(serializeCommandDefinition(definition))
            }
            
            FileWriter(commandsFile).use { writer ->
                gson.toJson(jsonArray, writer)
            }
            
        } catch (e: Exception) {
            logger.severe("Failed to save commands: ${e.message}")
        }
    }
    
    /**
     * Parse command definition from JSON
     */
    private fun parseCommandDefinition(json: JsonObject): CommandDefinition {
        val name = json.get("name")?.asString 
            ?: throw IllegalArgumentException("Command name is required")
        
        val description = json.get("description")?.asString ?: ""
        val usage = json.get("usage")?.asString ?: ""
        val permission = json.get("permission")?.asString
        val aliases = json.get("aliases")?.asJsonArray?.map { it.asString } ?: emptyList()
        
        val arguments = mutableListOf<ArgumentDefinition>()
        json.get("arguments")?.asJsonArray?.forEach { argElement ->
            arguments.add(parseArgumentDefinition(argElement.asJsonObject))
        }
        
        val subcommands = mutableListOf<SubcommandDefinition>()
        json.get("subcommands")?.asJsonArray?.forEach { subElement ->
            subcommands.add(parseSubcommandDefinition(subElement.asJsonObject))
        }
        
        val responses = mutableMapOf<String, String>()
        json.get("responses")?.asJsonObject?.entrySet()?.forEach { entry ->
            responses[entry.key] = entry.value.asString
        }
        
        return CommandDefinition(
            name = name,
            description = description,
            usage = usage,
            permission = permission,
            aliases = aliases,
            arguments = arguments,
            subcommands = subcommands,
            responses = responses
        )
    }
    
    /**
     * Parse argument definition from JSON
     */
    private fun parseArgumentDefinition(json: JsonObject): ArgumentDefinition {
        return ArgumentDefinition(
            name = json.get("name")?.asString ?: throw IllegalArgumentException("Argument name required"),
            type = ArgumentType.valueOf(json.get("type")?.asString?.uppercase() ?: "STRING"),
            required = json.get("required")?.asBoolean ?: true,
            defaultValue = json.get("default")?.asString,
            suggestions = json.get("suggestions")?.asJsonArray?.map { it.asString } ?: emptyList(),
            min = json.get("min")?.asDouble,
            max = json.get("max")?.asDouble,
            regex = json.get("regex")?.asString
        )
    }
    
    /**
     * Parse subcommand definition from JSON  
     */
    private fun parseSubcommandDefinition(json: JsonObject): SubcommandDefinition {
        val arguments = mutableListOf<ArgumentDefinition>()
        json.get("arguments")?.asJsonArray?.forEach { argElement ->
            arguments.add(parseArgumentDefinition(argElement.asJsonObject))
        }
        
        return SubcommandDefinition(
            name = json.get("name")?.asString ?: throw IllegalArgumentException("Subcommand name required"),
            description = json.get("description")?.asString ?: "",
            permission = json.get("permission")?.asString,
            arguments = arguments
        )
    }
    
    /**
     * Serialize command definition to JSON
     */
    private fun serializeCommandDefinition(definition: CommandDefinition): JsonObject {
        return JsonObject().apply {
            addProperty("name", definition.name)
            addProperty("description", definition.description)
            addProperty("usage", definition.usage)
            definition.permission?.let { addProperty("permission", it) }
            
            if (definition.aliases.isNotEmpty()) {
                val aliasArray = JsonArray()
                definition.aliases.forEach { aliasArray.add(it) }
                add("aliases", aliasArray)
            }
            
            if (definition.arguments.isNotEmpty()) {
                val argsArray = JsonArray()
                definition.arguments.forEach { arg ->
                    argsArray.add(serializeArgumentDefinition(arg))
                }
                add("arguments", argsArray)
            }
            
            if (definition.subcommands.isNotEmpty()) {
                val subsArray = JsonArray()
                definition.subcommands.forEach { sub ->
                    subsArray.add(serializeSubcommandDefinition(sub))
                }
                add("subcommands", subsArray)
            }
            
            if (definition.responses.isNotEmpty()) {
                val responsesObj = JsonObject()
                definition.responses.forEach { (key, value) ->
                    responsesObj.addProperty(key, value)
                }
                add("responses", responsesObj)
            }
        }
    }
    
    /**
     * Serialize argument definition to JSON
     */
    private fun serializeArgumentDefinition(arg: ArgumentDefinition): JsonObject {
        return JsonObject().apply {
            addProperty("name", arg.name)
            addProperty("type", arg.type.name.lowercase())
            addProperty("required", arg.required)
            arg.defaultValue?.let { addProperty("default", it) }
            
            if (arg.suggestions.isNotEmpty()) {
                val sugArray = JsonArray()
                arg.suggestions.forEach { sugArray.add(it) }
                add("suggestions", sugArray)
            }
            
            arg.min?.let { addProperty("min", it) }
            arg.max?.let { addProperty("max", it) }
            arg.regex?.let { addProperty("regex", it) }
        }
    }
    
    /**
     * Serialize subcommand definition to JSON
     */
    private fun serializeSubcommandDefinition(sub: SubcommandDefinition): JsonObject {
        return JsonObject().apply {
            addProperty("name", sub.name)
            addProperty("description", sub.description)
            sub.permission?.let { addProperty("permission", it) }
            
            if (sub.arguments.isNotEmpty()) {
                val argsArray = JsonArray()
                sub.arguments.forEach { arg ->
                    argsArray.add(serializeArgumentDefinition(arg))
                }
                add("arguments", argsArray)
            }
        }
    }
    
    /**
     * Send command execution event to Bcon server
     */
    private fun sendCommandExecutionEvent(execution: CommandExecution) {
        val eventData = JsonObject().apply {
            addProperty("command", execution.command)
            execution.subcommand?.let { addProperty("subcommand", it) }
            addProperty("sender", execution.sender.name)
            addProperty("senderType", execution.sender.type.name.lowercase())
            addProperty("timestamp", execution.timestamp)
            
            val argsObj = JsonObject()
            execution.arguments.forEach { (key, value) ->
                when (value) {
                    is String -> argsObj.addProperty(key, value)
                    is Number -> argsObj.addProperty(key, value)
                    is Boolean -> argsObj.addProperty(key, value)
                    else -> argsObj.addProperty(key, value.toString())
                }
            }
            add("arguments", argsObj)
        }
        
        adapter.sendEvent("custom_command_executed", eventData)
    }
    
    /**
     * Handle built-in command responses
     */
    private fun handleBuiltinResponse(definition: CommandDefinition, execution: CommandExecution) {
        // Check for built-in responses
        val responseKey = if (execution.subcommand != null) {
            "${execution.subcommand}_success"
        } else {
            "success"
        }
        
        val response = definition.responses[responseKey]
        if (response != null) {
            // Send response to command sender
            // Platform-specific implementation will handle this
        }
    }
    
    /**
     * Register command with platform-specific system
     * This method should be overridden by platform implementations
     */
    protected open fun registerPlatformCommand(definition: CommandDefinition) {
        // Platform-specific implementations will override this
        logger.info("Platform command registration not implemented for: ${definition.name}")
    }
}

// Data classes for command system

data class CommandDefinition(
    val name: String,
    val description: String,
    val usage: String,
    val permission: String?,
    val aliases: List<String>,
    val arguments: List<ArgumentDefinition>,
    val subcommands: List<SubcommandDefinition>,
    val responses: Map<String, String>
)

data class ArgumentDefinition(
    val name: String,
    val type: ArgumentType,
    val required: Boolean,
    val defaultValue: String?,
    val suggestions: List<String>,
    val min: Double?,
    val max: Double?,
    val regex: String?
)

data class SubcommandDefinition(
    val name: String,
    val description: String,
    val permission: String?,
    val arguments: List<ArgumentDefinition>
)

data class CommandExecution(
    val command: String,
    val subcommand: String?,
    val arguments: Map<String, Any>,
    val sender: CommandSender,
    val timestamp: Long
)

data class CommandSender(
    val name: String,
    val type: SenderType,
    val uuid: String? = null
)

enum class ArgumentType {
    STRING, INTEGER, DOUBLE, BOOLEAN, PLAYER, WORLD, ITEM, LOCATION, ENUM
}

enum class SenderType {
    PLAYER, CONSOLE, COMMAND_BLOCK
}