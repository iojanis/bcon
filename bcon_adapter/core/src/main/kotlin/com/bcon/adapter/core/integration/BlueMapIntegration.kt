package com.bcon.adapter.core.integration

import com.bcon.adapter.core.BconAdapter
import com.google.gson.Gson
import com.google.gson.JsonObject
import java.util.concurrent.ConcurrentHashMap

/**
 * Optional BlueMap integration for world visualization
 * Provides marker management and map interaction capabilities
 */
class BlueMapIntegration(private val adapter: BconAdapter) {
    
    private val logger = adapter.logger
    private val gson = Gson()
    private val markers = ConcurrentHashMap<String, BlueMapMarker>()
    
    private var blueMapApi: Any? = null
    private var isEnabled = false
    
    init {
        try {
            initializeBlueMapAPI()
            isEnabled = true
            logger.info("BlueMap integration initialized successfully")
        } catch (e: Exception) {
            logger.warning("BlueMap not available or failed to initialize: ${e.message}")
            isEnabled = false
        }
    }
    
    /**
     * Handle marker-related commands from Bcon server
     */
    fun handleMarkerCommand(data: JsonObject): String {
        if (!isEnabled) {
            return "BlueMap integration is not available"
        }
        
        return try {
            val action = data.get("action")?.asString ?: return "No action specified"
            
            when (action) {
                "add_marker" -> addMarker(data)
                "remove_marker" -> removeMarker(data.get("id")?.asString ?: "")
                "update_marker" -> updateMarker(data)
                "list_markers" -> listMarkers()
                "clear_markers" -> clearMarkers()
                "get_marker" -> getMarker(data.get("id")?.asString ?: "")
                else -> "Unknown marker action: $action"
            }
        } catch (e: Exception) {
            logger.severe("Error handling marker command: ${e.message}")
            "Error: ${e.message}"
        }
    }
    
    /**
     * Add a new marker to the BlueMap
     */
    private fun addMarker(data: JsonObject): String {
        val id = data.get("id")?.asString ?: return "Marker ID is required"
        val world = data.get("world")?.asString ?: return "World is required"
        val x = data.get("x")?.asDouble ?: return "X coordinate is required"
        val y = data.get("y")?.asDouble ?: return "Y coordinate is required" 
        val z = data.get("z")?.asDouble ?: return "Z coordinate is required"
        val label = data.get("label")?.asString ?: "Marker"
        val detail = data.get("detail")?.asString
        val icon = data.get("icon")?.asString ?: "assets/poi.svg"
        val anchor = parseAnchor(data.get("anchor")?.asJsonObject)
        
        val marker = BlueMapMarker(
            id = id,
            world = world,
            x = x,
            y = y,
            z = z,
            label = label,
            detail = detail,
            icon = icon,
            anchor = anchor
        )
        
        markers[id] = marker
        
        // Add marker to BlueMap (platform-specific implementation)
        addMarkerToBlueMap(marker)
        
        logger.info("Added BlueMap marker: $id at ($x, $y, $z) in $world")
        return "Marker '$id' added successfully"
    }
    
    /**
     * Remove a marker from BlueMap
     */
    private fun removeMarker(id: String): String {
        if (id.isEmpty()) {
            return "Marker ID is required"
        }
        
        val marker = markers.remove(id)
        return if (marker != null) {
            removeMarkerFromBlueMap(marker)
            logger.info("Removed BlueMap marker: $id")
            "Marker '$id' removed successfully"
        } else {
            "Marker '$id' not found"
        }
    }
    
    /**
     * Update an existing marker
     */
    private fun updateMarker(data: JsonObject): String {
        val id = data.get("id")?.asString ?: return "Marker ID is required"
        val existingMarker = markers[id] ?: return "Marker '$id' not found"
        
        val updatedMarker = existingMarker.copy(
            label = data.get("label")?.asString ?: existingMarker.label,
            detail = data.get("detail")?.asString ?: existingMarker.detail,
            icon = data.get("icon")?.asString ?: existingMarker.icon,
            x = data.get("x")?.asDouble ?: existingMarker.x,
            y = data.get("y")?.asDouble ?: existingMarker.y,
            z = data.get("z")?.asDouble ?: existingMarker.z
        )
        
        markers[id] = updatedMarker
        updateMarkerInBlueMap(updatedMarker)
        
        logger.info("Updated BlueMap marker: $id")
        return "Marker '$id' updated successfully"
    }
    
    /**
     * List all markers
     */
    private fun listMarkers(): String {
        if (markers.isEmpty()) {
            return "No markers found"
        }
        
        val markerList = markers.values.map { marker ->
            "${marker.id}: ${marker.label} at (${marker.x}, ${marker.y}, ${marker.z}) in ${marker.world}"
        }
        
        return "Markers (${markers.size}):\n${markerList.joinToString("\n")}"
    }
    
    /**
     * Clear all markers
     */
    private fun clearMarkers(): String {
        val count = markers.size
        markers.clear()
        clearAllMarkersFromBlueMap()
        
        logger.info("Cleared all BlueMap markers")
        return "Cleared $count markers"
    }
    
    /**
     * Get specific marker details
     */
    private fun getMarker(id: String): String {
        if (id.isEmpty()) {
            return "Marker ID is required"
        }
        
        val marker = markers[id]
        return if (marker != null) {
            gson.toJson(marker)
        } else {
            "Marker '$id' not found"
        }
    }
    
    /**
     * Parse anchor configuration from JSON
     */
    private fun parseAnchor(anchorData: JsonObject?): MarkerAnchor {
        return if (anchorData != null) {
            MarkerAnchor(
                x = anchorData.get("x")?.asInt ?: 25,
                y = anchorData.get("y")?.asInt ?: 45
            )
        } else {
            MarkerAnchor(25, 45) // Default anchor
        }
    }
    
    /**
     * Initialize BlueMap API (platform-specific)
     * This method should be overridden by platform implementations
     */
    protected open fun initializeBlueMapAPI() {
        // Platform-specific implementations will override this
        // For now, we'll simulate availability check
        val blueMapPresent = try {
            // Try to check if BlueMap classes are available
            Class.forName("de.bluecolored.bluemap.api.BlueMapAPI")
            true
        } catch (e: ClassNotFoundException) {
            false
        }
        
        if (!blueMapPresent) {
            throw IllegalStateException("BlueMap API not found in classpath")
        }
        
        logger.info("BlueMap API classes found, integration ready")
    }
    
    /**
     * Add marker to BlueMap (platform-specific)
     */
    protected open fun addMarkerToBlueMap(marker: BlueMapMarker) {
        // Platform-specific implementations will override this
        logger.fine("Platform marker addition not implemented for: ${marker.id}")
    }
    
    /**
     * Remove marker from BlueMap (platform-specific)
     */
    protected open fun removeMarkerFromBlueMap(marker: BlueMapMarker) {
        // Platform-specific implementations will override this
        logger.fine("Platform marker removal not implemented for: ${marker.id}")
    }
    
    /**
     * Update marker in BlueMap (platform-specific)
     */
    protected open fun updateMarkerInBlueMap(marker: BlueMapMarker) {
        // Platform-specific implementations will override this
        logger.fine("Platform marker update not implemented for: ${marker.id}")
    }
    
    /**
     * Clear all markers from BlueMap (platform-specific)
     */
    protected open fun clearAllMarkersFromBlueMap() {
        // Platform-specific implementations will override this
        logger.fine("Platform marker clearing not implemented")
    }
    
    /**
     * Check if BlueMap integration is available
     */
    fun isAvailable(): Boolean = isEnabled
    
    /**
     * Get current marker count
     */
    fun getMarkerCount(): Int = markers.size
    
    /**
     * Shutdown BlueMap integration
     */
    fun shutdown() {
        markers.clear()
        blueMapApi = null
        isEnabled = false
        logger.info("BlueMap integration shutdown")
    }
}

// Data classes for BlueMap integration

data class BlueMapMarker(
    val id: String,
    val world: String,
    val x: Double,
    val y: Double,
    val z: Double,
    val label: String,
    val detail: String? = null,
    val icon: String = "assets/poi.svg",
    val anchor: MarkerAnchor = MarkerAnchor(25, 45)
)

data class MarkerAnchor(
    val x: Int = 25,
    val y: Int = 45
)