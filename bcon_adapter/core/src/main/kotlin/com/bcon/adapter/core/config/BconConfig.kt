package com.bcon.adapter.core.config

import com.bcon.adapter.core.logging.BconLogger
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import com.google.gson.JsonObject
import java.io.File
import java.io.FileReader
import java.io.FileWriter

/**
 * Configuration management for Bcon adapter
 * Supports JSON configuration with automatic creation of default config
 */
class BconConfig(private val logger: BconLogger) {
    private val gson: Gson = GsonBuilder().setPrettyPrinting().create()
    private val configFile = File("plugins/bcon/config.json")
    
    // Configuration properties
    var jwtToken: String = ""
        private set
    var serverUrl: String = "ws://localhost:8082"
        private set
    var serverId: String = "minecraft_server_1"  
        private set
    var serverName: String = "Minecraft Server"
        private set
    var strictMode: Boolean = false
        private set
    var reconnectDelay: Int = 5000
        private set
    var enableBlueMap: Boolean = true
        private set
    var heartbeatInterval: Int = 30000
        private set
    var connectionTimeout: Int = 10000
        private set
    
    init {
        loadConfig()
    }
    
    /**
     * Load configuration from file or create default if not exists
     */
    fun loadConfig() {
        try {
            // Create config directory if it doesn't exist
            configFile.parentFile?.mkdirs()
            
            if (!configFile.exists()) {
                logger.info("Config file not found, creating default configuration")
                createDefaultConfig()
                return
            }
            
            logger.info("Loading configuration from ${configFile.absolutePath}")
            
            FileReader(configFile).use { reader ->
                val config = gson.fromJson(reader, JsonObject::class.java)
                
                // Load configuration values with defaults
                jwtToken = config.get("jwtToken")?.asString ?: ""
                serverUrl = config.get("serverUrl")?.asString ?: "ws://localhost:8082"
                serverId = config.get("serverId")?.asString ?: "minecraft_server_1"
                serverName = config.get("serverName")?.asString ?: "Minecraft Server"
                strictMode = config.get("strictMode")?.asBoolean ?: false
                reconnectDelay = config.get("reconnectDelayMs")?.asInt ?: 5000
                enableBlueMap = config.get("enableBlueMap")?.asBoolean ?: true
                heartbeatInterval = config.get("heartbeatIntervalMs")?.asInt ?: 30000
                connectionTimeout = config.get("connectionTimeoutMs")?.asInt ?: 10000
                
                validateConfig()
                logger.info("Configuration loaded successfully")
            }
            
        } catch (e: Exception) {
            logger.severe("Error loading configuration: ${e.message}")
            logger.info("Creating backup of current config and generating new default")
            
            // Backup problematic config
            try {
                if (configFile.exists()) {
                    val backupFile = File(configFile.parent, "config.json.backup")
                    configFile.copyTo(backupFile, overwrite = true)
                    logger.info("Backup created at ${backupFile.absolutePath}")
                }
            } catch (backupError: Exception) {
                logger.warning("Failed to create backup: ${backupError.message}")
            }
            
            // Create new default config
            createDefaultConfig()
        }
    }
    
    /**
     * Create default configuration file
     */
    private fun createDefaultConfig() {
        val defaultConfig = JsonObject().apply {
            addProperty("jwtToken", "YOUR_JWT_TOKEN_HERE")
            addProperty("serverUrl", "ws://localhost:8082")  
            addProperty("serverId", "minecraft_server_1")
            addProperty("serverName", "Minecraft Server")
            addProperty("strictMode", false)
            addProperty("reconnectDelayMs", 5000)
            addProperty("enableBlueMap", true)
            addProperty("heartbeatIntervalMs", 30000)
            addProperty("connectionTimeoutMs", 10000)
        }
        
        try {
            FileWriter(configFile).use { writer ->
                gson.toJson(defaultConfig, writer)
            }
            logger.info("Default configuration created at ${configFile.absolutePath}")
            logger.warning("Please edit the config file and set your JWT token!")
        } catch (e: Exception) {
            logger.severe("Failed to create default configuration: ${e.message}")
        }
    }
    
    /**
     * Save current configuration to file
     */
    private fun saveConfig() {
        try {
            // Create config directory if it doesn't exist
            configFile.parentFile?.mkdirs()
            
            val configJson = JsonObject().apply {
                addProperty("jwtToken", jwtToken)
                addProperty("serverUrl", serverUrl)
                addProperty("serverId", serverId)
                addProperty("serverName", serverName)
                addProperty("strictMode", strictMode)
                addProperty("reconnectDelayMs", reconnectDelay)
                addProperty("enableBlueMap", enableBlueMap)
                addProperty("heartbeatIntervalMs", heartbeatInterval)
                addProperty("connectionTimeoutMs", connectionTimeout)
            }
            
            FileWriter(configFile).use { writer ->
                gson.toJson(configJson, writer)
            }
            
            logger.info("Configuration saved to ${configFile.absolutePath}")
        } catch (e: Exception) {
            logger.severe("Failed to save configuration: ${e.message}")
            throw e
        }
    }
    
    /**
     * Validate configuration values
     */
    private fun validateConfig() {
        if (jwtToken.isEmpty() || jwtToken == "YOUR_JWT_TOKEN_HERE") {
            logger.warning("JWT token is not configured! Please set a valid token in config.json")
        }
        
        if (!serverUrl.startsWith("ws://") && !serverUrl.startsWith("wss://")) {
            logger.warning("Server URL should start with ws:// or wss://")
        }
        
        if (serverId.isEmpty()) {
            logger.warning("Server ID is empty - using default")
            serverId = "minecraft_server_1"
        }
        
        if (reconnectDelay < 1000) {
            logger.warning("Reconnect delay is very low, minimum 1000ms recommended")
        }
        
        if (heartbeatInterval < 10000) {
            logger.warning("Heartbeat interval is very low, minimum 10000ms recommended")
        }
    }
    
    /**
     * Save current configuration to file
     */
    fun save() {
        try {
            val config = JsonObject().apply {
                addProperty("jwtToken", jwtToken)
                addProperty("serverUrl", serverUrl)
                addProperty("serverId", serverId) 
                addProperty("serverName", serverName)
                addProperty("strictMode", strictMode)
                addProperty("reconnectDelayMs", reconnectDelay)
                addProperty("enableBlueMap", enableBlueMap)
                addProperty("heartbeatIntervalMs", heartbeatInterval)
                addProperty("connectionTimeoutMs", connectionTimeout)
            }
            
            FileWriter(configFile).use { writer ->
                gson.toJson(config, writer)
            }
            
            logger.info("Configuration saved successfully")
        } catch (e: Exception) {
            logger.severe("Failed to save configuration: ${e.message}")
        }
    }
    
    /**
     * Check if configuration is valid for connection
     */
    fun isValid(): Boolean {
        return jwtToken.isNotEmpty() && 
               jwtToken != "YOUR_JWT_TOKEN_HERE" && 
               serverUrl.isNotEmpty() &&
               serverId.isNotEmpty()
    }
    
    /**
     * Update JWT token and save configuration
     */
    fun updateJwtToken(newToken: String): Boolean {
        return try {
            jwtToken = newToken
            saveConfig()
            logger.info("JWT token updated successfully")
            true
        } catch (e: Exception) {
            logger.severe("Failed to update JWT token: ${e.message}")
            false
        }
    }
    
    /**
     * Update server URL and save configuration
     */
    fun updateServerUrl(newUrl: String): Boolean {
        return try {
            // Validate URL format
            if (!newUrl.startsWith("ws://") && !newUrl.startsWith("wss://")) {
                logger.warning("Invalid server URL format: $newUrl (must start with ws:// or wss://)")
                return false
            }
            
            serverUrl = newUrl
            saveConfig()
            logger.info("Server URL updated to: $serverUrl")
            true
        } catch (e: Exception) {
            logger.severe("Failed to update server URL: ${e.message}")
            false
        }
    }
    
    /**
     * Update server ID and save configuration
     */
    fun updateServerId(newId: String): Boolean {
        return try {
            if (newId.isBlank()) {
                logger.warning("Server ID cannot be blank")
                return false
            }
            
            serverId = newId
            saveConfig()
            logger.info("Server ID updated to: $serverId")
            true
        } catch (e: Exception) {
            logger.severe("Failed to update server ID: ${e.message}")
            false
        }
    }
    
    /**
     * Update server name and save configuration
     */
    fun updateServerName(newName: String): Boolean {
        return try {
            serverName = newName
            saveConfig()
            logger.info("Server name updated to: $serverName")
            true
        } catch (e: Exception) {
            logger.severe("Failed to update server name: ${e.message}")
            false
        }
    }
    
    /**
     * Update strict mode and save configuration
     */
    fun updateStrictMode(enabled: Boolean): Boolean {
        return try {
            strictMode = enabled
            saveConfig()
            logger.info("Strict mode ${if (enabled) "enabled" else "disabled"}")
            true
        } catch (e: Exception) {
            logger.severe("Failed to update strict mode: ${e.message}")
            false
        }
    }
}