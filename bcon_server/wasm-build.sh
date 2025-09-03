#!/bin/bash

# Build script for WASM target
echo "Building Bcon server for WASM target..."

# Install wasm-pack if not available
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build for WASM with web target
echo "Building WASM package..."
wasm-pack build --target web --out-dir pkg --features wasm

# Build for Node.js target
echo "Building Node.js package..."
wasm-pack build --target nodejs --out-dir pkg-node --features wasm

echo "WASM build complete!"
echo "Web package: ./pkg/"
echo "Node.js package: ./pkg-node/"

# Generate TypeScript definitions
echo "Generating TypeScript definitions..."
cat > pkg/index.d.ts << 'EOF'
/* tslint:disable */
/* eslint-disable */
/**
* WASM Bcon Server
*/
export class WasmBconServer {
  free(): void;
  constructor(config_json: string);
  start(): Promise<void>;
  get_metrics(): any;
  kv_set(key: string, value: string): void;
  kv_get(key: string): any;
  kv_delete(key: string): void;
}

export interface BconConfig {
  adapter_port: number;
  client_port: number;
  adapter_secret: string;
  client_secret: string;
  rate_limits: RateLimitConfig;
  allowed_origins: string[];
  heartbeat_interval_seconds: number;
  connection_timeout_seconds: number;
  log_level: string;
  server_info: ServerInfo;
}

export interface RateLimitConfig {
  guest_requests_per_minute: number;
  player_requests_per_minute: number;
  admin_requests_per_minute: number;
  system_requests_per_minute: number;
  unauthenticated_adapter_attempts_per_minute: number;
  window_duration_seconds: number;
  ban_threshold: number;
  ban_duration_hours: number;
}

export interface ServerInfo {
  name: string;
  description: string;
  url: string;
  minecraft_version: string;
}

export interface BconMetrics {
  active_adapters: number;
  active_clients: number;
  messages_per_second: number;
  connection_errors: number;
  authentication_failures: number;
}
EOF

echo "TypeScript definitions generated: pkg/index.d.ts"

# Create example usage files
cat > pkg/example-web.js << 'EOF'
import init, { WasmBconServer } from './bcon_server.js';

async function run() {
    await init();
    
    const config = {
        adapter_port: 8082,
        client_port: 8081,
        adapter_secret: "your_adapter_secret_here_minimum_32_chars",
        client_secret: "your_client_secret_here_minimum_32_chars",
        rate_limits: {
            guest_requests_per_minute: 30,
            player_requests_per_minute: 120,
            admin_requests_per_minute: 300,
            system_requests_per_minute: 1000,
            unauthenticated_adapter_attempts_per_minute: 5,
            window_duration_seconds: 60,
            ban_threshold: 50,
            ban_duration_hours: 24
        },
        allowed_origins: ["*"],
        heartbeat_interval_seconds: 30,
        connection_timeout_seconds: 300,
        log_level: "info",
        server_info: {
            name: "Bcon Server",
            description: "WebSocket communication server",
            url: "localhost",
            minecraft_version: "1.20+"
        }
    };

    const server = new WasmBconServer(JSON.stringify(config));
    
    // Start the server
    await server.start();
    
    // Get metrics
    const metrics = server.get_metrics();
    console.log('Server metrics:', metrics);
    
    // Use KV store
    server.kv_set("test_key", "test_value");
    const value = server.kv_get("test_key");
    console.log('Retrieved value:', value);
}

run().catch(console.error);
EOF

cat > pkg-node/example-node.js << 'EOF'
const { WasmBconServer } = require('./bcon_server.js');

async function run() {
    const config = {
        adapter_port: 8082,
        client_port: 8081,
        adapter_secret: "your_adapter_secret_here_minimum_32_chars",
        client_secret: "your_client_secret_here_minimum_32_chars",
        rate_limits: {
            guest_requests_per_minute: 30,
            player_requests_per_minute: 120,
            admin_requests_per_minute: 300,
            system_requests_per_minute: 1000,
            unauthenticated_adapter_attempts_per_minute: 5,
            window_duration_seconds: 60,
            ban_threshold: 50,
            ban_duration_hours: 24
        },
        allowed_origins: ["*"],
        heartbeat_interval_seconds: 30,
        connection_timeout_seconds: 300,
        log_level: "info",
        server_info: {
            name: "Bcon Server",
            description: "WebSocket communication server",
            url: "localhost",
            minecraft_version: "1.20+"
        }
    };

    const server = new WasmBconServer(JSON.stringify(config));
    
    try {
        console.log('Starting Bcon server...');
        await server.start();
        
        // Get metrics
        const metrics = server.get_metrics();
        console.log('Server metrics:', metrics);
        
        // Use KV store
        server.kv_set("test_key", "test_value");
        const value = server.kv_get("test_key");
        console.log('Retrieved value:', value);
        
    } catch (error) {
        console.error('Server error:', error);
    }
}

run();
EOF

echo "Example files created:"
echo "  Web: pkg/example-web.js"
echo "  Node.js: pkg-node/example-node.js"

# Create package.json for npm publishing
cat > pkg/package.json << EOF
{
  "name": "bcon-server-wasm",
  "version": "0.1.0",
  "description": "WebSocket communication server for Minecraft adapters and clients (WASM)",
  "main": "bcon_server.js",
  "types": "index.d.ts",
  "files": [
    "bcon_server.js",
    "bcon_server_bg.wasm",
    "bcon_server.d.ts",
    "index.d.ts"
  ],
  "keywords": ["websocket", "minecraft", "server", "wasm", "communication"],
  "author": "Bcon Team",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/bcon-server"
  }
}
EOF

cat > pkg-node/package.json << EOF
{
  "name": "bcon-server-node",
  "version": "0.1.0",
  "description": "WebSocket communication server for Minecraft adapters and clients (Node.js)",
  "main": "bcon_server.js",
  "types": "../pkg/index.d.ts",
  "files": [
    "bcon_server.js",
    "bcon_server_bg.wasm",
    "bcon_server.d.ts"
  ],
  "keywords": ["websocket", "minecraft", "server", "nodejs", "communication"],
  "author": "Bcon Team",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/your-org/bcon-server"
  }
}
EOF

echo "Package.json files created for npm publishing"
echo ""
echo "To use in web applications:"
echo "  import init, { WasmBconServer } from 'bcon-server-wasm';"
echo ""
echo "To use in Node.js/Deno:"
echo "  const { WasmBconServer } = require('bcon-server-node');"
echo "  // or"
echo "  import { WasmBconServer } from 'bcon-server-node';"