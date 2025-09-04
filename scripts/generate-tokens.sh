#!/bin/bash

# Token Generation Script for Bcon
# Generates all necessary authentication tokens for the complete setup

set -e

echo "🔐 Bcon Token Generator"
echo "======================"

# Load environment variables
if [ -f .env ]; then
    source .env
else
    echo "❌ .env file not found. Run ./scripts/setup.sh first."
    exit 1
fi

# Check if token generator exists
if [ ! -f "bcon_server/target/release/token_gen" ]; then
    echo "❌ Token generator not found. Building..."
    cd bcon_server
    cargo build --release --bin token_gen
    cd ..
fi

echo "🎯 Generating authentication tokens..."
echo ""

# Generate system client token
echo "📡 Generating system client token..."
SYSTEM_TOKEN=$(./bcon_server/target/release/token_gen --type client --role system --username "SystemClient" --quiet)
echo "   System Client Token: $SYSTEM_TOKEN"

# Generate adapter token
echo "🔌 Generating Minecraft adapter token..."
ADAPTER_TOKEN=$(./bcon_server/target/release/token_gen --type adapter --server-id "$MINECRAFT_SERVER_ID" --server-name "$MINECRAFT_SERVER_NAME" --quiet)
echo "   Adapter Token: $ADAPTER_TOKEN"

# Generate demo player token
echo "👤 Generating demo player token..."
PLAYER_TOKEN=$(./bcon_server/target/release/token_gen --type client --role player --username "DemoPlayer" --quiet)
echo "   Demo Player Token: $PLAYER_TOKEN"

echo ""
echo "💾 Updating configuration files..."

# Update .env file with tokens
sed -i.bak "s/SYSTEM_CLIENT_TOKEN=.*/SYSTEM_CLIENT_TOKEN=$SYSTEM_TOKEN/" .env
sed -i.bak "s/MINECRAFT_ADAPTER_TOKEN=.*/MINECRAFT_ADAPTER_TOKEN=$ADAPTER_TOKEN/" .env
sed -i.bak "s/DEMO_PLAYER_TOKEN=.*/DEMO_PLAYER_TOKEN=$PLAYER_TOKEN/" .env
rm .env.bak

# Update bcon-adapter config
sed -i.bak "s/\${MINECRAFT_ADAPTER_TOKEN}/$ADAPTER_TOKEN/" config/bcon-adapter.json
rm config/bcon-adapter.json.bak

echo "✅ All tokens generated and configurations updated!"
echo ""
echo "📋 Token Summary:"
echo "   🖥️  System Client: $SYSTEM_TOKEN"
echo "   🔌 Minecraft Adapter: $ADAPTER_TOKEN"
echo "   👤 Demo Player: $PLAYER_TOKEN"
echo ""
echo "💡 Usage Examples:"
echo "   # Connect as system client"
echo "   ./bcon_client/target/release/bcon_client system --token \"$SYSTEM_TOKEN\""
echo ""
echo "   # Connect as demo player"
echo "   ./bcon_client/target/release/bcon_client player --token \"$PLAYER_TOKEN\""
echo ""
echo "   # Minecraft server will use adapter token automatically"
echo ""
echo "🔄 Restart Docker services to apply new tokens:"
echo "   docker-compose down && docker-compose up -d"