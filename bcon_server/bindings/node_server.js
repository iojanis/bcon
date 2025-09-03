#!/usr/bin/env node

/**
 * Node.js EventEmitter wrapper for Bcon Server
 * 
 * This wrapper provides an EventEmitter interface around the Rust bcon_server
 * by spawning it as a child process and parsing its logs/output.
 */

const { EventEmitter } = require('events');
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * Node.js wrapper for Bcon Server with EventEmitter interface
 */
class BconServer extends EventEmitter {
    constructor(options = {}) {
        super();
        
        this.options = {
            adapterPort: options.adapterPort || 8082,
            clientPort: options.clientPort || 8081,
            logLevel: options.logLevel || 'info',
            configPath: options.configPath || null,
            ...options
        };
        
        this.process = null;
        this.isRunning = false;
        this.adapterListening = false;
        this.clientListening = false;
        this.serverStats = {
            adapters: 0,
            clients: 0,
            messages: 0,
            errors: 0
        };
        
        // Bind event handlers
        this.on('newListener', (event, listener) => {
            if (event === 'ready' && this.isRunning) {
                // Emit ready immediately if already running
                process.nextTick(() => listener());
            }
        });
    }
    
    /**
     * Start the Bcon server
     */
    async start() {
        if (this.isRunning) {
            throw new Error('Server is already running');
        }
        
        console.log('🚀 Starting Bcon Server...');
        
        // Find the bcon server binary
        const serverPath = await this.findServerBinary();
        if (!serverPath) {
            throw new Error('Bcon server binary not found. Please build it first with: cargo build --release');
        }
        
        // Create config if needed
        const configPath = await this.createConfig();
        
        // Start the server process
        const args = [];
        if (configPath) {
            args.push('--config', configPath);
        }
        
        console.log(`📡 Starting server: ${serverPath} ${args.join(' ')}`);
        
        this.process = spawn(serverPath, args, {
            stdio: ['pipe', 'pipe', 'pipe'],
            env: {
                ...process.env,
                RUST_LOG: this.options.logLevel
            }
        });
        
        // Handle process events
        this.process.on('error', (error) => {
            this.emit('error', new Error(`Failed to start server: ${error.message}`));
        });
        
        this.process.on('exit', (code, signal) => {
            this.isRunning = false;
            this.emit('shutdown', { code, signal });
            
            if (code !== 0 && code !== null) {
                this.emit('error', new Error(`Server exited with code ${code}`));
            }
        });
        
        // Parse stdout for events
        this.process.stdout.on('data', (data) => {
            this.parseServerOutput(data.toString());
        });
        
        // Parse stderr for events
        this.process.stderr.on('data', (data) => {
            this.parseServerOutput(data.toString());
        });
        
        // Wait for server to be ready
        return new Promise((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error('Server startup timeout'));
            }, 30000);
            
            this.once('ready', () => {
                clearTimeout(timeout);
                resolve();
            });
            
            this.once('error', (error) => {
                clearTimeout(timeout);
                reject(error);
            });
        });
    }
    
    /**
     * Stop the Bcon server
     */
    async stop() {
        if (!this.isRunning || !this.process) {
            return;
        }
        
        console.log('👋 Stopping Bcon Server...');
        
        return new Promise((resolve) => {
            this.process.once('exit', () => {
                resolve();
            });
            
            // Try graceful shutdown first
            this.process.kill('SIGTERM');
            
            // Force kill after 5 seconds
            setTimeout(() => {
                if (this.process && !this.process.killed) {
                    this.process.kill('SIGKILL');
                }
            }, 5000);
        });
    }
    
    /**
     * Get server statistics
     */
    getStats() {
        return { ...this.serverStats, isRunning: this.isRunning };
    }
    
    /**
     * Find the Bcon server binary
     */
    async findServerBinary() {
        const possiblePaths = [
            path.join(__dirname, '..', 'target', 'release', 'bcon'),
            path.join(__dirname, '..', 'target', 'debug', 'bcon'),
            path.join(process.cwd(), 'target', 'release', 'bcon'),
            path.join(process.cwd(), 'target', 'debug', 'bcon'),
        ];
        
        for (const binaryPath of possiblePaths) {
            try {
                await fs.promises.access(binaryPath, fs.constants.F_OK | fs.constants.X_OK);
                return binaryPath;
            } catch (e) {
                // Continue searching
            }
        }
        
        return null;
    }
    
    /**
     * Create server configuration
     */
    async createConfig() {
        if (this.options.configPath) {
            return this.options.configPath;
        }
        
        const config = {
            adapter_port: this.options.adapterPort,
            client_port: this.options.clientPort,
            adapter_secret: "adapter_test_secret_32_chars_minimum_length_required_here",
            client_secret: "client_test_secret_32_chars_minimum_length_required_here",
            rate_limits: {
                guest_requests_per_minute: 30,
                player_requests_per_minute: 120,
                admin_requests_per_minute: 300,
                system_requests_per_minute: 1000,
                unauthenticated_adapter_attempts_per_minute: 5,
                window_duration_seconds: 60,
                ban_threshold: 100,
                ban_duration_hours: 24
            },
            allowed_origins: [
                "http://localhost:3000",
                "https://yourserver.com"
            ],
            heartbeat_interval_seconds: 30,
            connection_timeout_seconds: 300,
            log_level: this.options.logLevel,
            server_info: {
                name: "Bcon Server (Node.js)",
                description: "Node.js wrapped Bcon server",
                url: "localhost",
                minecraft_version: "1.20.4"
            }
        };
        
        const configPath = path.join(__dirname, 'node_config.json');
        await fs.promises.writeFile(configPath, JSON.stringify(config, null, 2));
        return configPath;
    }
    
    /**
     * Parse server output for events
     */
    parseServerOutput(output) {
        const lines = output.split('\\n');
        
        for (const line of lines) {
            if (!line.trim()) continue;
            
            
            // Parse log levels and events
            if (line.includes('Starting Bcon server')) {
                console.log('🔄 Server starting...');
            }
            
            if (line.includes('listening on')) {
                if (line.includes('Adapter WebSocket server')) {
                    console.log(`🔌 Adapter server listening on port ${this.options.adapterPort}`);
                    this.adapterListening = true;
                }
                if (line.includes('Client WebSocket server')) {
                    console.log(`👥 Client server listening on port ${this.options.clientPort}`);
                    this.clientListening = true;
                }
                
                // Server is ready when both listeners are up
                if (this.adapterListening && this.clientListening && !this.isRunning) {
                    this.isRunning = true;
                    this.emit('ready');
                }
            }
            
            if (line.includes('connected')) {
                if (line.includes('adapter')) {
                    this.serverStats.adapters++;
                    this.emit('adapterConnected', { timestamp: new Date() });
                } else if (line.includes('client')) {
                    this.serverStats.clients++;
                    this.emit('clientConnected', { timestamp: new Date() });
                }
            }
            
            if (line.includes('disconnected')) {
                if (line.includes('adapter')) {
                    this.serverStats.adapters = Math.max(0, this.serverStats.adapters - 1);
                    this.emit('adapterDisconnected', { timestamp: new Date() });
                } else if (line.includes('client')) {
                    this.serverStats.clients = Math.max(0, this.serverStats.clients - 1);
                    this.emit('clientDisconnected', { timestamp: new Date() });
                }
            }
            
            if (line.includes('message')) {
                this.serverStats.messages++;
                this.emit('messageProcessed', { timestamp: new Date() });
            }
            
            if (line.includes('ERROR') || line.includes('error')) {
                this.serverStats.errors++;
                this.emit('error', new Error(`Server error: ${line}`));
            }
        }
    }
}

module.exports = BconServer;