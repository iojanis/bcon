package com.bcon.adapter.core.connection

import com.bcon.adapter.core.BconAdapter
import com.bcon.adapter.core.config.BconConfig
import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonSyntaxException
import java.net.URI
import java.net.http.HttpClient
import java.net.http.WebSocket
import java.nio.ByteBuffer
import java.time.Duration
import java.util.concurrent.*

/**
 * WebSocket client for connecting to Bcon server
 * Handles connection management, reconnection, and message routing
 */
class BconWebSocketClient(
    private val config: BconConfig,
    private val adapter: BconAdapter
) : WebSocket.Listener {
    
    private val logger = adapter.logger
    private val gson = Gson()
    private val messageBuffer = StringBuffer()
    private val bufferLock = Any()
    
    private var webSocket: WebSocket? = null
    private var httpClient: HttpClient? = null
    private var heartbeatExecutor: ScheduledExecutorService? = null
    private var connectionMonitor: ScheduledExecutorService? = null
    private var isShuttingDown = false
    private var lastPongReceived = System.currentTimeMillis()
    private var lastMessageReceived = System.currentTimeMillis() // Track any activity
    private var connectionAttempts = 0
    private val maxConnectionAttempts = 100 // Much higher limit for persistent reconnection
    
    /**
     * Initialize the WebSocket connection
     */
    fun initialize() {
        if (!config.isValid()) {
            logger.severe("Configuration is invalid! Cannot connect to Bcon server.")
            logger.severe("Please check your JWT token and server URL in config.json")
            if (config.strictMode) {
                throw IllegalStateException("Invalid configuration in strict mode")
            }
            return
        }
        
        logger.info("Initializing WebSocket connection to ${config.serverUrl}")
        connectWebSocket()
    }
    
    /**
     * Shutdown the WebSocket connection
     */
    fun shutdown() {
        isShuttingDown = true
        
        heartbeatExecutor?.shutdown()
        connectionMonitor?.shutdown()
        
        try {
            heartbeatExecutor?.awaitTermination(5, TimeUnit.SECONDS)
            connectionMonitor?.awaitTermination(5, TimeUnit.SECONDS)
        } catch (e: InterruptedException) {
            logger.warning("Executor shutdown interrupted")
        }
        
        webSocket?.sendClose(WebSocket.NORMAL_CLOSURE, "Shutting down")
        httpClient = null
        
        logger.info("WebSocket connection shutdown complete")
    }
    
    /**
     * Check if WebSocket is connected
     */
    fun isConnected(): Boolean {
        return webSocket != null && !isShuttingDown
    }
    
    /**
     * Send event to Bcon server
     */
    fun sendEvent(eventType: String, data: JsonObject?) {
        val webSocketInstance = webSocket
        if (webSocketInstance == null) {
            logger.warning("Cannot send event '$eventType' - WebSocket not connected")
            return
        }
        
        try {
            val message = JsonObject().apply {
                addProperty("eventType", eventType)
                add("data", data ?: JsonObject())
                addProperty("timestamp", System.currentTimeMillis() / 1000)
            }
            
            val jsonString = gson.toJson(message)
            webSocketInstance.sendText(jsonString, true).thenAccept {
                logger.fine("Sent event: $eventType")
            }.exceptionally { throwable ->
                logger.severe("⚠️  SEND EVENT FAILED: '$eventType' - ${throwable.message} - Connection lost, reconnecting immediately!")
                // Force immediate reconnection when send fails
                webSocket = null
                connectionAttempts = 0
                handleConnectionFailure()
                null
            }
            
        } catch (e: Exception) {
            logger.severe("⚠️  SEND EVENT EXCEPTION: '$eventType' - ${e.message} - Connection lost, reconnecting immediately!")
            // Force immediate reconnection when send throws exception
            webSocket = null
            connectionAttempts = 0
            handleConnectionFailure()
        }
    }
    
    /**
     * Establish WebSocket connection
     */
    private fun connectWebSocket() {
        try {
            httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofMillis(config.connectionTimeout.toLong()))
                .build()
            
            val uri = URI.create(config.serverUrl)
            
            val connectionFuture = httpClient!!.newWebSocketBuilder()
                .header("Authorization", "Bearer ${config.jwtToken}")
                .connectTimeout(Duration.ofMillis(config.connectionTimeout.toLong()))
                .buildAsync(uri, this)
            
            try {
                webSocket = connectionFuture.get(config.connectionTimeout.toLong(), TimeUnit.MILLISECONDS)
                logger.info("WebSocket connection established successfully")
            } catch (e: TimeoutException) {
                logger.severe("Connection timeout after ${config.connectionTimeout}ms")
                handleConnectionFailure()
            } catch (e: Exception) {
                logger.severe("Failed to establish WebSocket connection: ${e.message}")
                handleConnectionFailure()
            }
            
        } catch (e: Exception) {
            logger.severe("Failed to initialize WebSocket client: ${e.message}")
            handleConnectionFailure()
        }
    }
    
    /**
     * Handle connection failure with reconnection logic
     */
    private fun handleConnectionFailure() {
        webSocket = null
        connectionAttempts++
        
        if (isShuttingDown) {
            return
        }
        
        if (config.strictMode) {
            logger.severe("Connection failed in strict mode - requesting server shutdown")
            adapter.handleStrictModeFailure()
            return
        }
        
        if (connectionAttempts > maxConnectionAttempts) {
            logger.severe("Max connection attempts ($maxConnectionAttempts) exceeded - giving up")
            if (config.strictMode) {
                adapter.handleStrictModeFailure()
            }
            return
        }
        
        // Instant reconnection for the first few attempts, then gradual backoff
        val delay = when {
            connectionAttempts <= 5 -> 100L // Instant reconnection first 5 attempts (100ms)
            connectionAttempts <= 15 -> 1000L // 1 second for next 10 attempts
            connectionAttempts <= 30 -> 5000L // 5 seconds for next 15 attempts
            else -> 15000L // 15 seconds for remaining attempts
        }
        
        logger.warning("Connection failed (attempt $connectionAttempts/$maxConnectionAttempts), retrying in ${delay}ms")
        
        CompletableFuture.delayedExecutor(delay, TimeUnit.MILLISECONDS)
            .execute(::connectWebSocket)
    }
    
    /**
     * Start heartbeat mechanism
     */
    private fun startHeartbeat() {
        heartbeatExecutor?.shutdown()
        heartbeatExecutor = Executors.newSingleThreadScheduledExecutor { r ->
            Thread(r, "bcon-heartbeat").apply {
                isDaemon = true
            }
        }
        
        heartbeatExecutor!!.scheduleWithFixedDelay({
            try {
                sendHeartbeat()
            } catch (e: Exception) {
                logger.warning("Failed to send heartbeat: ${e.message}")
            }
        }, config.heartbeatInterval.toLong(), config.heartbeatInterval.toLong(), TimeUnit.MILLISECONDS)
        
        logger.info("Heartbeat started with ${config.heartbeatInterval}ms interval")
    }
    
    /**
     * Send heartbeat ping
     */
    private fun sendHeartbeat() {
        try {
            webSocket?.sendPing(ByteBuffer.allocate(0))?.thenAccept { 
                logger.fine("Heartbeat sent successfully")
            }?.exceptionally { throwable ->
                logger.severe("⚠️  HEARTBEAT FAILED: ${throwable.message} - Connection lost, reconnecting immediately!")
                // Force immediate reconnection when heartbeat fails
                webSocket = null
                connectionAttempts = 0
                handleConnectionFailure()
                null
            }
        } catch (e: Exception) {
            logger.severe("⚠️  HEARTBEAT ERROR: ${e.message} - Connection lost, reconnecting immediately!")
            webSocket = null
            connectionAttempts = 0
            handleConnectionFailure()
        }
    }
    
    /**
     * Start connection monitoring to detect dead connections
     */
    private fun startConnectionMonitoring() {
        connectionMonitor?.shutdown()
        connectionMonitor = Executors.newSingleThreadScheduledExecutor { r ->
            Thread(r, "bcon-connection-monitor").apply {
                isDaemon = true
            }
        }
        
        // Check connection health every 30 seconds (more aggressive monitoring)
        connectionMonitor!!.scheduleWithFixedDelay({
            try {
                checkConnectionHealth()
            } catch (e: Exception) {
                logger.severe("Connection monitoring error: ${e.message}")
            }
        }, 30000, 30000, TimeUnit.MILLISECONDS)
        
        logger.info("Connection monitoring started")
    }
    
    /**
     * Check if connection is healthy by examining activity
     */
    private fun checkConnectionHealth() {
        val now = System.currentTimeMillis()
        val timeSinceLastMessage = now - lastMessageReceived
        val timeSinceLastPong = now - lastPongReceived
        
        // Shorter timeouts for faster detection - 2 minutes without activity
        val activityTimeout = 120000L // 2 minutes
        // Pong timeout - 3 minutes  
        val pongTimeout = 180000L // 3 minutes
        
        // Check if we have recent message activity OR pong responses
        val hasRecentActivity = timeSinceLastMessage < activityTimeout
        val hasRecentPong = timeSinceLastPong < pongTimeout
        
        if (!hasRecentActivity && !hasRecentPong) {
            logger.warning("Connection appears dead (no activity for ${timeSinceLastMessage / 1000}s, no pong for ${timeSinceLastPong / 1000}s) - forcing reconnection")
            
            // Force close the connection to trigger reconnection
            webSocket?.sendClose(WebSocket.NORMAL_CLOSURE, "Connection timeout")
            handleConnectionFailure()
        } else if (!hasRecentActivity) {
            logger.warning("No recent messages for ${timeSinceLastMessage / 1000}s but connection seems alive (pong ${timeSinceLastPong / 1000}s ago)")
        } else {
            logger.fine("Connection healthy (activity ${timeSinceLastMessage / 1000}s ago, pong ${timeSinceLastPong / 1000}s ago)")
        }
    }
    
    // WebSocket.Listener implementation
    
    override fun onOpen(webSocket: WebSocket) {
        logger.info("✅ BCON CONNECTION ESTABLISHED - Server monitoring active!")
        this.webSocket = webSocket
        connectionAttempts = 0 // Reset connection attempts on successful connection
        val now = System.currentTimeMillis()
        lastPongReceived = now
        lastMessageReceived = now
        startHeartbeat()
        startConnectionMonitoring()
        webSocket.request(1)
    }
    
    override fun onText(webSocket: WebSocket, data: CharSequence, last: Boolean): CompletionStage<*>? {
        synchronized(bufferLock) {
            messageBuffer.append(data)
            
            if (last) {
                try {
                    lastMessageReceived = System.currentTimeMillis() // Update activity timestamp
                    val completeMessage = messageBuffer.toString()
                    handleIncomingMessage(completeMessage)
                } catch (e: Exception) {
                    logger.severe("Error processing message: ${e.message}")
                    logger.severe("Message content: ${messageBuffer}")
                } finally {
                    messageBuffer.setLength(0)
                }
            }
        }
        
        webSocket.request(1)
        return null
    }
    
    override fun onClose(webSocket: WebSocket, statusCode: Int, reason: String): CompletionStage<*>? {
        logger.severe("⚠️  BCON CONNECTION LOST: $statusCode $reason - Attempting immediate reconnection!")
        this.webSocket = null
        // Force immediate reconnection attempt by resetting connection attempts
        connectionAttempts = 0
        handleConnectionFailure()
        return null
    }
    
    override fun onError(webSocket: WebSocket, error: Throwable) {
        logger.severe("⚠️  BCON CONNECTION ERROR: ${error.message} - Attempting immediate reconnection!")
        this.webSocket = null
        // Force immediate reconnection attempt by resetting connection attempts
        connectionAttempts = 0
        handleConnectionFailure()
    }
    
    override fun onPing(webSocket: WebSocket, message: ByteBuffer): CompletionStage<*>? {
        logger.fine("Received ping")
        return null
    }
    
    override fun onPong(webSocket: WebSocket, message: ByteBuffer): CompletionStage<*>? {
        lastPongReceived = System.currentTimeMillis()
        logger.fine("Received pong - connection alive")
        return null
    }
    
    /**
     * Handle incoming message from Bcon server
     */
    private fun handleIncomingMessage(message: String) {
        if (message.trim().isEmpty()) {
            logger.warning("Received empty message")
            return
        }
        
        try {
            val jsonMessage = gson.fromJson(message, JsonObject::class.java)
            
            // Handle the new server message format
            val messageId = jsonMessage.get("messageId")?.asString
            val eventType = jsonMessage.get("type")?.asString
            val data = jsonMessage.get("data")?.asJsonObject
            val requiresAck = jsonMessage.get("requiresAck")?.asBoolean ?: false
            
            if (messageId == null || eventType == null) {
                logger.warning("Received malformed message without required fields: $message")
                return
            }
            
            logger.info("Processing command: $eventType (id: $messageId, requires_ack: $requiresAck)")
            
            // Process command asynchronously to avoid blocking
            CompletableFuture.runAsync {
                try {
                    val result = adapter.handleIncomingCommand(eventType, data)
                    
                    // Send acknowledgment if required
                    if (requiresAck) {
                        sendResponse(messageId, result)
                    }
                } catch (e: Exception) {
                    logger.severe("Error executing command '$eventType': ${e.message}")
                    
                    // Send error acknowledgment if required
                    if (requiresAck) {
                        sendErrorResponse(messageId, e.message ?: "Unknown error")
                    }
                }
            }
            
        } catch (e: JsonSyntaxException) {
            logger.severe("JSON parsing error: ${e.message}")
            logger.severe("Problematic message: $message")
        } catch (e: Exception) {
            logger.severe("Error handling message: ${e.message}")
        }
    }
    
    /**
     * Send success response to Bcon server
     */
    private fun sendResponse(messageId: String, result: String) {
        try {
            val response = JsonObject().apply {
                addProperty("eventType", "command_result")
                addProperty("replyTo", messageId)
                addProperty("timestamp", System.currentTimeMillis() / 1000)
                add("data", JsonObject().apply {
                    addProperty("success", true)
                    addProperty("result", result)
                })
            }
            
            val jsonString = gson.toJson(response)
            webSocket?.sendText(jsonString, true)
            
            logger.fine("Sent acknowledgment for command: $messageId")
        } catch (e: Exception) {
            logger.severe("Failed to send response: ${e.message}")
        }
    }
    
    /**
     * Send error response to Bcon server
     */
    private fun sendErrorResponse(messageId: String, error: String) {
        try {
            val response = JsonObject().apply {
                addProperty("eventType", "command_result")
                addProperty("replyTo", messageId)
                addProperty("timestamp", System.currentTimeMillis() / 1000)
                add("data", JsonObject().apply {
                    addProperty("success", false)
                    addProperty("error", error)
                })
            }
            
            val jsonString = gson.toJson(response)
            webSocket?.sendText(jsonString, true)
            
            logger.warning("Sent error acknowledgment for command: $messageId - Error: $error")
        } catch (e: Exception) {
            logger.severe("Failed to send error response: ${e.message}")
        }
    }
}