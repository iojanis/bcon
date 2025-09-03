#!/usr/bin/env -S deno run --allow-net --allow-read

/**
 * Deno Example for Bcon Client
 * 
 * This example demonstrates how to use the Bcon client in Deno
 * using the WASM bindings.
 * 
 * Usage:
 *   deno run --allow-net --allow-read deno_client.ts [server_url] [role] [token]
 * 
 * Examples:
 *   deno run --allow-net --allow-read deno_client.ts
 *   deno run --allow-net --allow-read deno_client.ts ws://localhost:8081 guest
 *   deno run --allow-net --allow-read deno_client.ts ws://localhost:8081 player your-token
 */

import init, { WasmBconClientBuilder } from '../pkg/bcon_client.js';

interface TestMessage {
    eventType: string;
    data: Record<string, any>;
    messageId: string;
    timestamp: number;
}

async function main() {
    console.log('🚀 Starting Bcon Deno Client Example');
    
    // Initialize WASM module
    console.log('📦 Initializing WASM module...');
    await init();
    console.log('✅ WASM module initialized');
    
    // Get command line arguments
    const args = Deno.args;
    const serverUrl = args[0] || 'ws://localhost:8081';
    const role = args[1] || 'guest';
    const token = args[2];
    
    console.log(`📡 Connecting to: ${serverUrl}`);
    console.log(`👤 Role: ${role}`);
    
    try {
        // Create client builder
        console.log('🔧 Creating client builder...');
        let builder = new WasmBconClientBuilder(serverUrl);
        
        // Configure timeouts
        builder = builder.with_timeout(30000);
        builder = builder.with_heartbeat_interval(30000);
        
        // Add authentication if needed (must be done after other configurations)
        if (role !== 'guest' && token) {
            console.log('🔐 Adding authentication...');
            builder = builder.with_auth_token(token, role);
        }
        
        // Build the client
        console.log('🔧 Building client...');
        const client = builder.build();
        
        console.log('✅ Client created successfully');
        console.log('📦 Client state:', client.get_state_string());
        
        // Connect to server
        console.log('🔌 Connecting to server...');
        await client.connect();
        
        console.log('✅ Connected successfully');
        console.log('📦 Client state:', client.get_state_string());
        
        // Test message sending
        console.log('📤 Sending test message...');
        const testMessage: TestMessage = {
            eventType: 'test',
            data: { 
                message: 'Hello from Deno!', 
                timestamp: Date.now(),
                deno_version: Deno.version.deno,
                platform: Deno.build.os
            },
            messageId: crypto.randomUUID(),
            timestamp: Date.now()
        };
        
        client.send_js_message(JSON.stringify(testMessage));
        console.log('✅ Test message sent');
        
        // Send heartbeat
        console.log('💓 Sending heartbeat...');
        const heartbeat: TestMessage = {
            eventType: 'heartbeat',
            data: {},
            messageId: crypto.randomUUID(),
            timestamp: Date.now()
        };
        
        client.send_js_message(JSON.stringify(heartbeat));
        console.log('✅ Heartbeat sent');
        
        // Send server info request if we're authenticated
        if (role !== 'guest') {
            console.log('ℹ️  Requesting server info...');
            const serverInfoRequest: TestMessage = {
                eventType: 'get_server_info',
                data: {},
                messageId: crypto.randomUUID(),
                timestamp: Date.now()
            };
            
            client.send_js_message(JSON.stringify(serverInfoRequest));
            console.log('✅ Server info request sent');
        }
        
        // Keep alive for a bit
        console.log('⏳ Keeping connection alive for 5 seconds...');
        await new Promise(resolve => setTimeout(resolve, 5000));
        
        // Disconnect
        console.log('👋 Disconnecting...');
        await client.disconnect();
        
        console.log('✅ Disconnected successfully');
        console.log('🏁 Example completed');
        
    } catch (error) {
        console.error('❌ Error:', error.message || error);
        Deno.exit(1);
    }
}

// Handle process termination
Deno.addSignalListener('SIGINT', () => {
    console.log('\\n👋 Shutting down gracefully...');
    Deno.exit(0);
});

Deno.addSignalListener('SIGTERM', () => {
    console.log('\\n👋 Shutting down gracefully...');
    Deno.exit(0);
});

if (import.meta.main) {
    await main().catch(error => {
        console.error('❌ Unhandled error:', error);
        Deno.exit(1);
    });
}