package com.bcon.adapter.core

import com.bcon.adapter.core.config.BconConfig
import com.bcon.adapter.core.connection.BconWebSocketClient
import com.bcon.adapter.core.events.EventManager
import com.bcon.adapter.core.commands.DynamicCommandManager
import com.bcon.adapter.core.integration.BlueMapIntegration
import com.bcon.adapter.core.logging.BconLogger
import com.bcon.adapter.core.logging.JavaBconLogger
import com.google.gson.JsonObject

/**
 * Core Bcon adapter class that handles WebSocket connection and event routing
 * Designed to be platform-agnostic and extended by platform-specific implementations
 */
abstract class BconAdapter {
    
    open val logger: BconLogger = JavaBconLogger(this::class.java.simpleName)
    lateinit var config: BconConfig
    protected lateinit var webSocketClient: BconWebSocketClient
    lateinit var eventManager: EventManager
    lateinit var commandManager: DynamicCommandManager
    protected var blueMapIntegration: BlueMapIntegration? = null
    
    /**
     * Initialize the adapter with configuration
     */
    fun initialize() {
        logger.info("Initializing Bcon Adapter")
        
        // Load configuration
        config = BconConfig(logger)
        
        // Initialize components
        eventManager = EventManager(this)
        commandManager = DynamicCommandManager(this)
        webSocketClient = BconWebSocketClient(config, this)
        
        // Initialize optional BlueMap integration
        try {
            blueMapIntegration = BlueMapIntegration(this)
            logger.info("BlueMap integration enabled")
        } catch (e: Exception) {
            logger.info("BlueMap not available, integration disabled")
        }
        
        // Platform-specific initialization
        onInitialize()
        
        // Register all events
        registerEvents()
        
        // Initialize WebSocket connection
        webSocketClient.initialize()
        
        logger.info("Bcon Adapter initialized successfully")
    }
    
    /**
     * Shutdown the adapter gracefully
     */
    fun shutdown() {
        logger.info("Shutting down Bcon Adapter")
        
        webSocketClient.shutdown()
        commandManager.shutdown()
        onShutdown()
        
        logger.info("Bcon Adapter shutdown complete")
    }
    
    /**
     * Send event to the Bcon server
     */
    fun sendEvent(eventType: String, data: JsonObject?) {
        webSocketClient.sendEvent(eventType, data)
    }
    
    /**
     * Handle incoming command from Bcon server
     */
    fun handleIncomingCommand(type: String, data: JsonObject?): String {
        return when (type) {
            "command" -> executeCommand(data?.get("command")?.asString ?: "")
            "chat" -> broadcastMessage(data?.get("message")?.asString ?: "")
            "register_command" -> {
                data?.let { commandManager.registerCommand(it) }
                "Command registered successfully"
            }
            "unregister_command" -> {
                commandManager.unregisterCommand(data?.get("name")?.asString ?: "")
                "Command unregistered successfully"  
            }
            "clear_commands" -> {
                commandManager.clearCommands()
                "All commands cleared successfully"
            }
            "bluemap" -> {
                blueMapIntegration?.handleMarkerCommand(data ?: JsonObject()) ?: "BlueMap not available"
            }
            "bcon_config" -> {
                handleBconConfigCommand(data ?: JsonObject())
            }
            else -> "Unknown command type: $type"
        }
    }
    
    /**
     * Handle built-in Bcon configuration commands
     */
    fun handleBconConfigCommand(data: JsonObject): String {
        val action = data.get("action")?.asString ?: return "Missing action parameter"
        
        return when (action) {
            "status" -> {
                "Bcon Status:\n" +
                "- Server URL: ${config.serverUrl}\n" +
                "- Server ID: ${config.serverId}\n" +
                "- Server Name: ${config.serverName}\n" +
                "- Strict Mode: ${config.strictMode}\n" +
                "- Connection: ${if (webSocketClient.isConnected()) "Connected" else "Disconnected"}\n" +
                "- Config Valid: ${config.isValid()}"
            }
            "token" -> {
                val newToken = data.get("value")?.asString 
                if (newToken.isNullOrBlank()) {
                    return "Usage: /bcon token <jwt_token>"
                }
                if (config.updateJwtToken(newToken)) {
                    // Restart connection with new token
                    webSocketClient.shutdown()
                    webSocketClient.initialize()
                    "JWT token updated and connection restarted"
                } else {
                    "Failed to update JWT token"
                }
            }
            "url" -> {
                val newUrl = data.get("value")?.asString 
                if (newUrl.isNullOrBlank()) {
                    return "Usage: /bcon url <ws://host:port>"
                }
                if (config.updateServerUrl(newUrl)) {
                    // Restart connection with new URL
                    webSocketClient.shutdown()
                    webSocketClient.initialize()
                    "Server URL updated and connection restarted"
                } else {
                    "Failed to update server URL"
                }
            }
            "id" -> {
                val newId = data.get("value")?.asString 
                if (newId.isNullOrBlank()) {
                    return "Usage: /bcon id <server_id>"
                }
                if (config.updateServerId(newId)) {
                    // Restart connection to register with new ID
                    webSocketClient.shutdown()
                    webSocketClient.initialize()
                    "Server ID updated and connection restarted"
                } else {
                    "Failed to update server ID"
                }
            }
            "name" -> {
                val newName = data.get("value")?.asString 
                if (newName.isNullOrBlank()) {
                    return "Usage: /bcon name <server_name>"
                }
                if (config.updateServerName(newName)) {
                    "Server name updated to: $newName"
                } else {
                    "Failed to update server name"
                }
            }
            "strict" -> {
                val value = data.get("value")?.asString?.lowercase() 
                val enabled = when (value) {
                    "true", "on", "yes", "1" -> true
                    "false", "off", "no", "0" -> false
                    else -> return "Usage: /bcon strict <true|false>"
                }
                if (config.updateStrictMode(enabled)) {
                    "Strict mode ${if (enabled) "enabled" else "disabled"}"
                } else {
                    "Failed to update strict mode"
                }
            }
            "reconnect" -> {
                webSocketClient.shutdown()
                webSocketClient.initialize()
                "Connection restarted"
            }
            "reload" -> {
                config.loadConfig()
                webSocketClient.shutdown()
                webSocketClient.initialize()
                "Configuration reloaded and connection restarted"
            }
            else -> {
                "Unknown action: $action\n" +
                "Available actions: status, token, url, id, name, strict, reconnect, reload"
            }
        }
    }
    
    // Abstract methods to be implemented by platform-specific adapters
    
    /**
     * Platform-specific initialization
     */
    protected abstract fun onInitialize()
    
    /**
     * Platform-specific shutdown
     */
    protected abstract fun onShutdown()
    
    /**
     * Register platform-specific events
     */
    protected abstract fun registerEvents()
    
    /**
     * Execute a server command
     */
    protected abstract fun executeCommand(command: String): String
    
    /**
     * Broadcast a message to all players
     */
    protected abstract fun broadcastMessage(message: String): String
    
    /**
     * Get the server instance (platform-specific)
     */
    abstract fun getServerInstance(): Any?
    
    /**
     * Check if the server is running
     */
    abstract fun isServerRunning(): Boolean
    
    /**
     * Get server information
     */
    abstract fun getServerInfo(): JsonObject
    
    /**
     * Handle strict mode connection failure (platform-specific server shutdown)
     */
    abstract fun handleStrictModeFailure()
}