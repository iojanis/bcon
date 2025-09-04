package com.bcon.adapter.folia.integration

import com.bcon.adapter.core.integration.BlueMapMarker
import org.bukkit.plugin.java.JavaPlugin
import java.util.logging.Logger

/**
 * Folia BlueMap integration - temporarily disabled due to API compatibility issues
 */
class FoliaBlueMapIntegration(private val plugin: JavaPlugin) {
    
    private val logger = Logger.getLogger(FoliaBlueMapIntegration::class.java.simpleName)
    
    init {
        logger.info("Folia BlueMap integration initialized (BlueMap API temporarily disabled)")
    }
    
    fun initializeBlueMapAPI() {
        logger.warning("BlueMap API integration temporarily disabled due to compatibility issues")
    }
    
    fun addMarkerToBlueMap(marker: BlueMapMarker) {
        logger.warning("BlueMap marker operations temporarily disabled")
    }
    
    fun removeMarkerFromBlueMap(marker: BlueMapMarker) {
        logger.warning("BlueMap marker operations temporarily disabled")
    }
    
    fun updateMarkerInBlueMap(marker: BlueMapMarker) {
        logger.warning("BlueMap marker operations temporarily disabled")
    }
    
    fun clearAllMarkersFromBlueMap() {
        logger.warning("BlueMap marker operations temporarily disabled")
    }
    
    fun shutdown() {
        logger.info("Folia BlueMap integration shutdown")
    }
    
    fun isBlueMapConnected(): Boolean = false
    
    fun getBlueMapInfo(): String = "BlueMap integration temporarily disabled"
}