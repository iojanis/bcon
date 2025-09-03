#!/usr/bin/env -S deno run --allow-read --allow-write --allow-run

/**
 * Deno EventTarget wrapper for Bcon Server
 * 
 * This wrapper provides an EventTarget interface around the Rust bcon_server
 * by spawning it as a subprocess and parsing its logs/output.
 */

import * as path from "https://deno.land/std@0.218.2/path/mod.ts";
import { existsSync } from "https://deno.land/std@0.218.2/fs/exists.ts";

export interface BconServerOptions {
    adapterPort?: number;
    clientPort?: number;
    logLevel?: string;
    configPath?: string;
}

export interface ServerStats {
    adapters: number;
    clients: number;
    messages: number;
    errors: number;
    isRunning: boolean;
}

/**
 * Deno wrapper for Bcon Server with EventTarget interface
 */
export class BconServer extends EventTarget {
    private options: Required<BconServerOptions>;
    private process: Deno.ChildProcess | null = null;
    private isRunning = false;
    private adapterListening = false;
    private clientListening = false;
    private serverStats: Omit<ServerStats, 'isRunning'> = {
        adapters: 0,
        clients: 0,
        messages: 0,
        errors: 0
    };
    
    constructor(options: BconServerOptions = {}) {
        super();
        
        this.options = {
            adapterPort: options.adapterPort ?? 8082,
            clientPort: options.clientPort ?? 8081,
            logLevel: options.logLevel ?? 'info',
            configPath: options.configPath ?? ''
        };
    }
    
    /**
     * Start the Bcon server
     */
    async start(): Promise<void> {
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
        
        const command = new Deno.Command(serverPath, {
            args,
            stdout: 'piped',
            stderr: 'piped',
            env: {
                RUST_LOG: this.options.logLevel
            }
        });
        
        this.process = command.spawn();
        
        // Handle process events
        this.process.status.then((status) => {
            this.isRunning = false;
            this.dispatchEvent(new CustomEvent('shutdown', { detail: { code: status.code } }));
            
            if (!status.success) {
                this.dispatchEvent(new CustomEvent('error', {
                    detail: new Error(`Server exited with code ${status.code}`)
                }));
            }
        }).catch((error) => {
            this.dispatchEvent(new CustomEvent('error', {
                detail: new Error(`Failed to start server: ${error.message}`)
            }));
        });
        
        // Parse stdout for events
        if (this.process.stdout) {
            this.readOutput(this.process.stdout);
        }
        
        // Parse stderr for events
        if (this.process.stderr) {
            this.readOutput(this.process.stderr);
        }
        
        // Wait for server to be ready
        return new Promise<void>((resolve, reject) => {
            const timeout = setTimeout(() => {
                reject(new Error('Server startup timeout'));
            }, 30000);
            
            const readyHandler = () => {
                clearTimeout(timeout);
                this.removeEventListener('ready', readyHandler);
                this.removeEventListener('error', errorHandler);
                resolve();
            };
            
            const errorHandler = (event: Event) => {
                clearTimeout(timeout);
                this.removeEventListener('ready', readyHandler);
                this.removeEventListener('error', errorHandler);
                reject((event as CustomEvent).detail);
            };
            
            this.addEventListener('ready', readyHandler);
            this.addEventListener('error', errorHandler);
        });
    }
    
    /**
     * Stop the Bcon server
     */
    async stop(): Promise<void> {
        if (!this.isRunning || !this.process) {
            return;
        }
        
        console.log('👋 Stopping Bcon Server...');
        
        // Try graceful shutdown first
        this.process.kill('SIGTERM');
        
        // Wait for exit or force kill after 5 seconds
        const timeoutId = setTimeout(() => {
            if (this.process) {
                this.process.kill('SIGKILL');
            }
        }, 5000);
        
        try {
            await this.process.status;
        } finally {
            clearTimeout(timeoutId);
        }
    }
    
    /**
     * Get server statistics
     */
    getStats(): ServerStats {
        return { ...this.serverStats, isRunning: this.isRunning };
    }
    
    /**
     * Find the Bcon server binary
     */
    private async findServerBinary(): Promise<string | null> {
        const possiblePaths = [
            path.join(import.meta.dirname ?? '.', '..', 'target', 'release', 'bcon'),
            path.join(import.meta.dirname ?? '.', '..', 'target', 'debug', 'bcon'),
            path.join(Deno.cwd(), 'target', 'release', 'bcon'),
            path.join(Deno.cwd(), 'target', 'debug', 'bcon'),
        ];
        
        for (const binaryPath of possiblePaths) {
            try {
                if (existsSync(binaryPath)) {
                    const fileInfo = await Deno.stat(binaryPath);
                    if (fileInfo.mode && (fileInfo.mode & 0o111)) { // Check if executable
                        return binaryPath;
                    }
                }
            } catch {
                // Continue searching
            }
        }
        
        return null;
    }
    
    /**
     * Create server configuration
     */
    private async createConfig(): Promise<string | null> {
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
                name: "Bcon Server (Deno)",
                description: "Deno wrapped Bcon server",
                url: "localhost",
                minecraft_version: "1.20.4"
            }
        };
        
        const configPath = path.join(import.meta.dirname ?? '.', 'deno_config.json');
        await Deno.writeTextFile(configPath, JSON.stringify(config, null, 2));
        return configPath;
    }
    
    /**
     * Read and parse output from stdout/stderr
     */
    private async readOutput(readable: ReadableStream<Uint8Array>): Promise<void> {
        const reader = readable.getReader();
        const decoder = new TextDecoder();
        
        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                
                const text = decoder.decode(value);
                this.parseServerOutput(text);
            }
        } catch (error) {
            this.dispatchEvent(new CustomEvent('error', {
                detail: new Error(`Failed to read server output: ${error.message}`)
            }));
        } finally {
            reader.releaseLock();
        }
    }
    
    /**
     * Parse server output for events
     */
    private parseServerOutput(output: string): void {
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
                    this.dispatchEvent(new CustomEvent('ready'));
                }
            }
            
            if (line.includes('connected')) {
                if (line.includes('adapter')) {
                    this.serverStats.adapters++;
                    this.dispatchEvent(new CustomEvent('adapterConnected', {
                        detail: { timestamp: new Date() }
                    }));
                } else if (line.includes('client')) {
                    this.serverStats.clients++;
                    this.dispatchEvent(new CustomEvent('clientConnected', {
                        detail: { timestamp: new Date() }
                    }));
                }
            }
            
            if (line.includes('disconnected')) {
                if (line.includes('adapter')) {
                    this.serverStats.adapters = Math.max(0, this.serverStats.adapters - 1);
                    this.dispatchEvent(new CustomEvent('adapterDisconnected', {
                        detail: { timestamp: new Date() }
                    }));
                } else if (line.includes('client')) {
                    this.serverStats.clients = Math.max(0, this.serverStats.clients - 1);
                    this.dispatchEvent(new CustomEvent('clientDisconnected', {
                        detail: { timestamp: new Date() }
                    }));
                }
            }
            
            if (line.includes('message')) {
                this.serverStats.messages++;
                this.dispatchEvent(new CustomEvent('messageProcessed', {
                    detail: { timestamp: new Date() }
                }));
            }
            
            if (line.includes('ERROR') || line.includes('error')) {
                this.serverStats.errors++;
                this.dispatchEvent(new CustomEvent('error', {
                    detail: new Error(`Server error: ${line}`)
                }));
            }
        }
    }
}