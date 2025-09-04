# Bcon Server Setup Guide

This guide walks you through setting up and running the Bcon Server, the central WebSocket relay hub that connects Minecraft adapters with client applications.

## Quick Start

### 1. Install Prerequisites
- **Rust 1.70+**: [Install Rust](https://rustup.rs/)
- **Git**: For cloning the repository

### 2. Clone and Build
```bash
git clone https://github.com/your-org/bcon.git
cd bcon/bcon_server

# Build the server
cargo build --release

# Build the token generator
cargo build --release --bin token_gen
```

### 3. Create Configuration
```bash
# Generate example configuration
./target/release/bcon --generate-config config.json

# Edit the configuration file
nano config.json
```

### 4. Generate Secrets
Generate secure secrets for JWT token signing:
```bash
# Generate random 32+ character secrets
openssl rand -hex 32  # Use for adapter_secret
openssl rand -hex 32  # Use for client_secret
```

Update your `config.json`:
```json
{
  "adapter_port": 8082,
  "client_port": 8081,
  "adapter_secret": "your_adapter_secret_here",
  "client_secret": "your_client_secret_here"
}
```

### 5. Start the Server
```bash
./target/release/bcon --config config.json
```

The server will start listening on:
- **Port 8082**: Minecraft adapter connections
- **Port 8081**: Client connections (web apps, bots, etc.)

## Configuration Reference

### Basic Configuration
```json
{
  "adapter_port": 8082,
  "client_port": 8081,
  "adapter_secret": "32_character_minimum_secret",
  "client_secret": "32_character_minimum_secret",
  "log_level": "info"
}
```

### Rate Limiting
```json
{
  "rate_limits": {
    "guest_requests_per_minute": 30,
    "player_requests_per_minute": 120,
    "admin_requests_per_minute": 300,
    "system_requests_per_minute": 1000,
    "unauthenticated_adapter_attempts_per_minute": 5,
    "window_duration_seconds": 60,
    "ban_threshold": 100,
    "ban_duration_hours": 24
  }
}
```

### Connection Management
```json
{
  "heartbeat_interval_seconds": 30,
  "connection_timeout_seconds": 300,
  "max_connections_per_ip": 10
}
```

## Token Management

### Generate System Client Token
For applications that need full access:
```bash
./target/release/token_gen \
  --type client \
  --role system \
  --username "MyApp" \
  --expires-in-days 30
```

### Generate Player Token  
For individual players with limited access:
```bash
./target/release/token_gen \
  --type client \
  --role player \
  --username "PlayerName" \
  --expires-in-days 7
```

### Generate Adapter Token
For Minecraft servers:
```bash
./target/release/token_gen \
  --type adapter \
  --server-id "my_server_1" \
  --server-name "My Minecraft Server" \
  --expires-in-days 365
```

### Verify Token
```bash
./target/release/token_gen --verify --token "your_token_here"
```

## Development Setup

### Debug Mode
```bash
RUST_LOG=debug ./target/release/bcon --config config.json
```

### Hot Reload Configuration
The server watches for configuration file changes and reloads automatically:
```bash
# Edit config.json in another terminal
# Server will automatically reload
```

### Testing
```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test integration

# Test with example clients
cargo run --example websocket_client
```

## Production Deployment

### Systemd Service
Create `/etc/systemd/system/bcon-server.service`:
```ini
[Unit]
Description=Bcon WebSocket Server
After=network.target

[Service]
Type=simple
User=bcon
WorkingDirectory=/opt/bcon-server
ExecStart=/opt/bcon-server/target/release/bcon --config /opt/bcon-server/config.json
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable bcon-server
sudo systemctl start bcon-server
sudo systemctl status bcon-server
```

### Docker Deployment
```dockerfile
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY target/release/bcon /app/bcon
COPY target/release/token_gen /app/token_gen
COPY config.json /app/config.json

EXPOSE 8081 8082

CMD ["./bcon", "--config", "config.json"]
```

### Load Balancing
For high-availability setups:

**nginx.conf**:
```nginx
upstream bcon_adapters {
    server bcon1:8082;
    server bcon2:8082;
    server bcon3:8082;
}

upstream bcon_clients {
    server bcon1:8081;
    server bcon2:8081;
    server bcon3:8081;
}

server {
    listen 8082;
    location / {
        proxy_pass http://bcon_adapters;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}

server {
    listen 8081;
    location / {
        proxy_pass http://bcon_clients;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

## Monitoring and Logging

### Health Checks
```bash
# Check server status
curl http://localhost:8081/health

# Monitor connections
curl http://localhost:8081/stats
```

### Log Analysis
```bash
# Follow live logs
tail -f server.log

# Filter by level
grep "ERROR" server.log
grep "WARN" server.log

# Monitor connection events
grep "connection" server.log | tail -20
```

### Metrics (if enabled)
```bash
# Prometheus metrics endpoint
curl http://localhost:8081/metrics
```

## Troubleshooting

### Common Issues

**❌ "Address already in use"**
```bash
# Check what's using the port
lsof -i :8081
lsof -i :8082

# Kill conflicting processes
sudo kill -9 <pid>
```

**❌ "Invalid JWT secret"**
```bash
# Secrets must be 32+ characters
# Regenerate with:
openssl rand -hex 32
```

**❌ "Connection refused"**
```bash
# Check server is running
ps aux | grep bcon

# Check firewall
sudo ufw status
sudo ufw allow 8081
sudo ufw allow 8082

# Check binding
netstat -an | grep LISTEN | grep -E "(8081|8082)"
```

**❌ High memory usage**
```bash
# Monitor memory
top -p $(pgrep bcon)

# Reduce connection limits in config:
{
  "max_connections_per_ip": 5,
  "connection_timeout_seconds": 60
}
```

### Performance Tuning

**For high-traffic servers**:
```json
{
  "rate_limits": {
    "system_requests_per_minute": 10000,
    "window_duration_seconds": 10
  },
  "heartbeat_interval_seconds": 15,
  "max_connections_per_ip": 50
}
```

**For resource-constrained servers**:
```json
{
  "rate_limits": {
    "guest_requests_per_minute": 10,
    "player_requests_per_minute": 30
  },
  "connection_timeout_seconds": 120,
  "max_connections_per_ip": 3
}
```

## Security Best Practices

### JWT Security
- **Strong Secrets**: Use 64+ character random secrets
- **Regular Rotation**: Rotate secrets monthly
- **Separate Secrets**: Use different secrets for adapters and clients
- **Expiration**: Set reasonable token expiration times

### Network Security
```json
{
  "bind_address": "127.0.0.1",  // Localhost only
  "allowed_origins": [
    "https://yourdomain.com",
    "https://dashboard.yourdomain.com"
  ],
  "rate_limits": {
    "ban_duration_hours": 168  // 1 week bans
  }
}
```

### Production Checklist
- [ ] Strong JWT secrets (64+ chars)
- [ ] Firewall configured (only necessary ports open)
- [ ] Rate limiting enabled
- [ ] Connection timeouts set
- [ ] Log rotation configured
- [ ] Monitoring set up
- [ ] Backup strategy for configuration
- [ ] SSL/TLS termination (nginx/cloudflare)

## API Reference

### WebSocket Connection
```javascript
// Client connection (port 8081)
const ws = new WebSocket('ws://localhost:8081');
ws.send(JSON.stringify({
  eventType: 'auth',
  data: { token: 'your_client_token' }
}));

// Adapter connection (port 8082) 
const ws = new WebSocket('ws://localhost:8082', {
  headers: { Authorization: 'Bearer your_adapter_token' }
});
```

### Message Format
```json
{
  "eventType": "player_joined",
  "data": {
    "playerId": "uuid",
    "playerName": "Steve",
    "x": 100.0,
    "y": 64.0, 
    "z": 200.0
  },
  "timestamp": 1699123456
}
```

## Support

- **GitHub Issues**: [Report bugs](https://github.com/your-org/bcon/issues)
- **Documentation**: [Full API docs](https://bcon-docs.example.com)
- **Discord**: [Community support](https://discord.gg/bcon)