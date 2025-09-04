#!/bin/bash

# Bcon Complete Setup Script
# This script sets up the entire Bcon ecosystem with Docker

set -e

echo "🚀 Bcon Complete Setup"
echo "======================"

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Create .env file if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env file from template..."
    cp .env.example .env
    
    echo "🔐 Generating secure secrets..."
    # Generate random secrets
    ADAPTER_SECRET=$(openssl rand -hex 32)
    CLIENT_SECRET=$(openssl rand -hex 32)
    
    # Update .env with generated secrets
    sed -i.bak "s/your_32_character_minimum_adapter_secret_here_change_this/$ADAPTER_SECRET/" .env
    sed -i.bak "s/your_32_character_minimum_client_secret_here_change_this/$CLIENT_SECRET/" .env
    rm .env.bak
    
    echo "✅ Secrets generated and saved to .env"
    echo ""
    echo "⚠️  IMPORTANT: You need to generate authentication tokens!"
    echo "   After building, run: ./scripts/generate-tokens.sh"
    echo ""
fi

# Load environment variables
source .env

echo "🔨 Building Bcon components..."

# Build server
echo "   Building Bcon Server..."
cd bcon_server
if [ ! -f "target/release/bcon" ]; then
    cargo build --release
fi
if [ ! -f "target/release/token_gen" ]; then
    cargo build --release --bin token_gen
fi
cd ..

# Build client  
echo "   Building Bcon Client..."
cd bcon_client
if [ ! -f "target/release/bcon_client" ]; then
    cargo build --release
fi
cd ..

echo "✅ Rust components built successfully"

# Create Dockerfiles if they don't exist
echo "🐳 Setting up Docker configurations..."

# Bcon Server Dockerfile
cat > bcon_server/Dockerfile << 'EOF'
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/bcon /app/bcon
COPY target/release/token_gen /app/token_gen
RUN chmod +x /app/bcon /app/token_gen

EXPOSE 8081 8082

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

CMD ["./bcon", "--config", "config.json"]
EOF

# Bcon Client Dockerfile
cat > bcon_client/Dockerfile << 'EOF'
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY target/release/bcon_client /app/bcon_client
RUN chmod +x /app/bcon_client

ENTRYPOINT ["./bcon_client"]
EOF

echo "✅ Docker configurations created"

echo "🎯 Starting services..."
docker-compose up -d --build

echo ""
echo "🎉 Bcon setup complete!"
echo ""
echo "Services running:"
echo "  📡 Bcon Server: localhost:${BCON_CLIENT_PORT:-8081} (clients) & localhost:${BCON_ADAPTER_PORT:-8082} (adapters)"
echo "  🎮 Minecraft Server: localhost:${MINECRAFT_PORT:-25565}"
echo "  🖥️  System Client: Streaming events in background"
echo ""
echo "Next steps:"
echo "  1. Generate authentication tokens: ./scripts/generate-tokens.sh"
echo "  2. Connect to Minecraft: localhost:${MINECRAFT_PORT:-25565}"
echo "  3. Watch live events: docker-compose logs -f bcon-system-client"
echo ""
echo "For more information, see GUIDE.md"