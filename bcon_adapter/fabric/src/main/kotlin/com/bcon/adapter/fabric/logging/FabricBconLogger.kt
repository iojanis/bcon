package com.bcon.adapter.fabric.logging

import com.bcon.adapter.core.logging.BconLogger
import org.slf4j.Logger
import org.slf4j.LoggerFactory

/**
 * Fabric-specific logger that integrates with Minecraft's logging system
 */
class FabricBconLogger(name: String) : BconLogger {
    private val logger: Logger = LoggerFactory.getLogger("BCON/$name")
    
    override fun info(message: String) = logger.info(message)
    override fun warning(message: String) = logger.warn(message)
    override fun error(message: String) = logger.error(message)
    override fun debug(message: String) = logger.debug(message)
    override fun fine(message: String) = logger.debug(message)
    override fun severe(message: String) = logger.error(message)
}