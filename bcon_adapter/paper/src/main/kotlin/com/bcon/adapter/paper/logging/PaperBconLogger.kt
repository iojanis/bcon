package com.bcon.adapter.paper.logging

import com.bcon.adapter.core.logging.BconLogger
import java.util.logging.Logger

/**
 * Paper/Bukkit-specific logger that integrates with Minecraft's logging system
 */
class PaperBconLogger(private val plugin: org.bukkit.plugin.Plugin) : BconLogger {
    private val logger: Logger = plugin.logger
    
    override fun info(message: String) = logger.info(message)
    override fun warning(message: String) = logger.warning(message)
    override fun error(message: String) = logger.severe(message)
    override fun debug(message: String) = logger.fine(message)
    override fun fine(message: String) = logger.fine(message)
    override fun severe(message: String) = logger.severe(message)
}