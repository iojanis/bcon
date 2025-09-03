# Multi-stage Docker build for Bcon Server

# Build stage
FROM rust:1.75-slim-bullseye as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --bin bcon
RUN rm -rf src

# Copy source code
COPY src ./src

# Build the actual binary
RUN touch src/main.rs && cargo build --release --bin bcon

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

# Create non-root user
RUN useradd --create-home --shell /bin/bash --uid 1000 bcon

# Copy binary from builder
COPY --from=builder /app/target/release/bcon /usr/local/bin/bcon

# Set ownership and permissions
RUN chmod +x /usr/local/bin/bcon

# Create config directory
RUN mkdir -p /etc/bcon && chown -R bcon:bcon /etc/bcon

# Create data directory for logs/temp files
RUN mkdir -p /var/lib/bcon && chown -R bcon:bcon /var/lib/bcon

# Switch to non-root user
USER bcon
WORKDIR /home/bcon

# Expose ports
EXPOSE 8081 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Default configuration via environment variables
ENV BCON_ADAPTER_PORT=8082
ENV BCON_CLIENT_PORT=8081
ENV BCON_LOG_LEVEL=info
ENV RUST_LOG=bcon_server=info

# Default command
CMD ["bcon"]

# Build metadata
LABEL org.opencontainers.image.title="Bcon Server"
LABEL org.opencontainers.image.description="WebSocket communication server for Minecraft adapters and clients"
LABEL org.opencontainers.image.source="https://github.com/your-org/bcon-server"
LABEL org.opencontainers.image.documentation="https://github.com/your-org/bcon-server/blob/main/README.md"
LABEL org.opencontainers.image.version="0.1.0"