use anyhow::{Result, Context};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

/// RCON client configuration
#[derive(Debug, Clone)]
pub struct RconConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub timeout: Duration,
}

impl Default for RconConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 25575,
            password: String::new(),
            timeout: Duration::from_secs(10),
        }
    }
}

/// Simple RCON connection implementation
struct SimpleRconConnection {
    stream: TcpStream,
    request_id: i32,
}

/// RCON packet types
const RCON_SERVERDATA_AUTH: i32 = 3;
const RCON_SERVERDATA_EXECCOMMAND: i32 = 2;
const RCON_SERVERDATA_AUTH_RESPONSE: i32 = 2;
const RCON_SERVERDATA_RESPONSE_VALUE: i32 = 0;

impl SimpleRconConnection {
    /// Create a new RCON connection
    async fn connect(address: &str, password: &str) -> Result<Self> {
        let stream = TcpStream::connect(address).await
            .context("Failed to connect to RCON server")?;
        
        let mut connection = Self {
            stream,
            request_id: 1,
        };
        
        // Authenticate
        connection.send_packet(RCON_SERVERDATA_AUTH, password).await?;
        let response = connection.read_packet().await?;
        
        if response.request_id != connection.request_id - 1 {
            return Err(anyhow::anyhow!("RCON authentication failed"));
        }
        
        Ok(connection)
    }
    
    /// Execute a command
    async fn execute(&mut self, command: &str) -> Result<String> {
        self.send_packet(RCON_SERVERDATA_EXECCOMMAND, command).await?;
        let response = self.read_packet().await?;
        Ok(response.body)
    }
    
    /// Send RCON packet
    async fn send_packet(&mut self, packet_type: i32, body: &str) -> Result<()> {
        let body_bytes = body.as_bytes();
        let packet_size = 4 + 4 + body_bytes.len() + 2; // request_id + type + body + 2 null bytes
        
        let mut packet = Vec::new();
        WriteBytesExt::write_i32::<LittleEndian>(&mut packet, packet_size as i32)?;
        WriteBytesExt::write_i32::<LittleEndian>(&mut packet, self.request_id)?;
        WriteBytesExt::write_i32::<LittleEndian>(&mut packet, packet_type)?;
        packet.extend_from_slice(body_bytes);
        WriteBytesExt::write_u8(&mut packet, 0)?; // null terminator for body
        WriteBytesExt::write_u8(&mut packet, 0)?; // null terminator for string
        
        self.stream.write_all(&packet).await?;
        self.request_id += 1;
        
        Ok(())
    }
    
    /// Read RCON packet
    async fn read_packet(&mut self) -> Result<RconPacket> {
        let size = self.stream.read_i32_le().await?;
        let request_id = self.stream.read_i32_le().await?;
        let packet_type = self.stream.read_i32_le().await?;
        
        let body_size = (size - 10) as usize; // size - request_id - type - 2 null bytes
        let mut body_bytes = vec![0u8; body_size];
        self.stream.read_exact(&mut body_bytes).await?;
        
        // Skip the two null terminators
        self.stream.read_u8().await?;
        self.stream.read_u8().await?;
        
        let body = String::from_utf8_lossy(&body_bytes).trim_end_matches('\0').to_string();
        
        Ok(RconPacket {
            request_id,
            packet_type,
            body,
        })
    }
}

/// RCON packet structure
struct RconPacket {
    request_id: i32,
    packet_type: i32,
    body: String,
}

/// RCON client wrapper that maintains persistent connections
pub struct RconClient {
    config: RconConfig,
    connection: Arc<RwLock<Option<SimpleRconConnection>>>,
}

impl RconClient {
    /// Create a new RCON client
    pub fn new(config: RconConfig) -> Self {
        Self {
            config,
            connection: Arc::new(RwLock::new(None)),
        }
    }

    /// Test if RCON connection is possible
    pub async fn test_connection(&self) -> Result<bool> {
        if self.config.password.is_empty() {
            debug!("RCON password is empty - RCON disabled");
            return Ok(false);
        }

        let address = format!("{}:{}", self.config.host, self.config.port);
        match timeout(
            self.config.timeout,
            SimpleRconConnection::connect(&address, &self.config.password)
        ).await {
            Ok(Ok(mut conn)) => {
                // Test with a simple command
                match timeout(self.config.timeout, conn.execute("list")).await {
                    Ok(Ok(_)) => {
                        info!("RCON connection test successful");
                        Ok(true)
                    }
                    Ok(Err(e)) => {
                        warn!("RCON command failed: {}", e);
                        Ok(false)
                    }
                    Err(_) => {
                        warn!("RCON command test timed out");
                        Ok(false)
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("RCON connection test failed: {}", e);
                Ok(false)
            }
            Err(_) => {
                debug!("RCON connection test timed out");
                Ok(false)
            }
        }
    }

    /// Get a connection, creating one if necessary  
    async fn get_connection(&self) -> Result<SimpleRconConnection> {
        let address = format!("{}:{}", self.config.host, self.config.port);
        debug!("Creating RCON connection to {}", address);
        
        let conn = timeout(
            self.config.timeout,
            SimpleRconConnection::connect(&address, &self.config.password)
        ).await
            .context("RCON connection timed out")?
            .context("Failed to connect to RCON")?;

        debug!("RCON connection established to {}", address);
        Ok(conn)
    }

    /// Execute a command via RCON with timeout
    pub async fn execute_command(&self, command: &str) -> Result<String> {
        if self.config.password.is_empty() {
            return Err(anyhow::anyhow!("RCON is disabled (no password configured)"));
        }

        debug!("Executing RCON command: {}", command);

        let mut conn = self.get_connection().await
            .context("Failed to get RCON connection")?;

        let result = timeout(
            self.config.timeout,
            conn.execute(command)
        ).await
            .context("RCON command timed out")?
            .context("RCON command failed")?;

        debug!("RCON command result: {}", result);
        Ok(result)
    }

    /// Check if RCON is enabled (has password)
    pub fn is_enabled(&self) -> bool {
        !self.config.password.is_empty()
    }

    /// Disconnect RCON client
    pub async fn disconnect(&self) {
        let mut connection_write = self.connection.write().await;
        if let Some(_conn) = connection_write.take() {
            info!("RCON connection closed");
        }
    }
}

/// RCON manager that handles multiple server connections
pub struct RconManager {
    clients: Arc<RwLock<std::collections::HashMap<String, Arc<RconClient>>>>,
}

impl RconManager {
    /// Create a new RCON manager
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Register an RCON client for a server
    pub async fn register_client(&self, server_id: String, config: RconConfig) -> Result<()> {
        let client = Arc::new(RconClient::new(config));
        
        // Test the connection
        if client.test_connection().await? {
            info!("RCON enabled for server: {}", server_id);
            let mut clients = self.clients.write().await;
            clients.insert(server_id, client);
        } else {
            debug!("RCON connection test failed for server: {} - RCON will be disabled", server_id);
        }
        
        Ok(())
    }

    /// Remove an RCON client
    pub async fn unregister_client(&self, server_id: &str) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.remove(server_id) {
            client.disconnect().await;
            info!("RCON client removed for server: {}", server_id);
        }
    }

    /// Execute a command via RCON for a specific server
    pub async fn execute_command(&self, server_id: &str, command: &str) -> Result<String> {
        let clients = self.clients.read().await;
        let client = clients.get(server_id)
            .ok_or_else(|| anyhow::anyhow!("No RCON client for server: {}", server_id))?;
        
        client.execute_command(command).await
    }

    /// Check if RCON is available for a server
    pub async fn is_rcon_available(&self, server_id: &str) -> bool {
        let clients = self.clients.read().await;
        clients.get(server_id)
            .map(|client| client.is_enabled())
            .unwrap_or(false)
    }

    /// Get all servers with RCON enabled
    pub async fn get_rcon_servers(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }

    /// Shutdown all RCON clients
    pub async fn shutdown(&self) {
        let mut clients = self.clients.write().await;
        for (_server_id, client) in clients.drain() {
            client.disconnect().await;
        }
        info!("All RCON clients shutdown");
    }
}

impl Default for RconManager {
    fn default() -> Self {
        Self::new()
    }
}