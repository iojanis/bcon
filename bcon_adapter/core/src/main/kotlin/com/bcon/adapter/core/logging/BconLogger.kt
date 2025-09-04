package com.bcon.adapter.core.logging

/**
 * Platform-agnostic logger interface for Bcon adapters
 */
interface BconLogger {
    fun info(message: String)
    fun warning(message: String)
    fun error(message: String)
    fun debug(message: String)
    fun fine(message: String)
    fun severe(message: String)
}

/**
 * Default Java logger implementation
 */
class JavaBconLogger(private val name: String) : BconLogger {
    private val logger = java.util.logging.Logger.getLogger(name)
    
    override fun info(message: String) = logger.info(message)
    override fun warning(message: String) = logger.warning(message)
    override fun error(message: String) = logger.severe(message)
    override fun debug(message: String) = logger.fine(message)
    override fun fine(message: String) = logger.fine(message)
    override fun severe(message: String) = logger.severe(message)
}