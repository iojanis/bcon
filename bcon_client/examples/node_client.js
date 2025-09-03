#!/usr/bin/env node

/**
 * Node.js Example for Bcon Client
 * 
 * This example demonstrates how to use the Bcon client in Node.js
 * using the WASM bindings.
 */

const { WasmBconClientBuilder } = require('../pkg-node/bcon_client.js');

async function main() {
    console.log('🚀 Starting Bcon Node.js Client Example');
    
    // Get command line arguments
    const args = process.argv.slice(2);
    const serverUrl = args[0] || 'ws://localhost:8081';
    const role = args[1] || 'guest';
    const token = args[2];
    
    console.log(`📡 Connecting to: ${serverUrl}`);
    console.log(`👤 Role: ${role}`);
    
    try {
        // Create client builder
        let builder = new WasmBconClientBuilder(serverUrl);
        
        // Configure timeouts
        builder = builder.with_timeout(30000.0);
        builder = builder.with_heartbeat_interval(30000.0);
        
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
        const testMessage = {
            eventType: 'test',
            data: { message: 'Hello from Node.js!', timestamp: Date.now() },
            messageId: Math.random().toString(36).substring(2),
            timestamp: Date.now()
        };
        
        client.send_js_message(JSON.stringify(testMessage));
        console.log('✅ Test message sent');
        
        // Send heartbeat
        console.log('💓 Sending heartbeat...');
        const heartbeat = {
            eventType: 'heartbeat',
            data: {},
            messageId: Math.random().toString(36).substring(2),
            timestamp: Date.now()
        };
        
        client.send_js_message(JSON.stringify(heartbeat));
        console.log('✅ Heartbeat sent');
        
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
        process.exit(1);
    }
}

// Handle process termination
process.on('SIGINT', () => {
    console.log('\n👋 Shutting down gracefully...');
    process.exit(0);
});

process.on('SIGTERM', () => {
    console.log('\n👋 Shutting down gracefully...');
    process.exit(0);
});

if (require.main === module) {
    main().catch(error => {
        console.error('❌ Unhandled error:', error);
        process.exit(1);
    });
}