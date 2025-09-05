use crate::auth::ClientRole;
use crate::command_tracker::CommandTracker;
use crate::connection::ConnectionManager;
use crate::kv_store::KvStore;
use crate::message::{IncomingMessage, OutgoingMessage, RelayMessage};
use crate::rcon_client::{RconManager, RconConfig};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub struct MessageRouter {
    connection_manager: Arc<ConnectionManager>,
    kv_store: Arc<KvStore>,
    command_tracker: Arc<CommandTracker>,
    rcon_manager: Arc<RconManager>,
    message_count: Arc<AtomicU64>,
}

impl MessageRouter {
    pub fn new(connection_manager: Arc<ConnectionManager>, kv_store: Arc<KvStore>, command_tracker: Arc<CommandTracker>, rcon_manager: Arc<RconManager>) -> Self {
        Self {
            connection_manager,
            kv_store,
            command_tracker,
            rcon_manager,
            message_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Route message from adapter to system clients ONLY
    /// System clients are responsible for deciding what to forward to other clients
    pub async fn route_adapter_message(
        &self,
        server_id: String,
        message: IncomingMessage,
    ) -> Result<()> {
        self.message_count.fetch_add(1, Ordering::Relaxed);
        
        // Combined logging moved to after routing

        // Create relay message with server context
        let relay_message = RelayMessage::new(
            message.event_type.clone(),
            message.data.clone(),
            Some(server_id.clone()),
        );

        let outgoing_message = OutgoingMessage::new(
            message.event_type.clone(),
            serde_json::to_value(&relay_message)?,
        ).with_message_id(
            message.message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        );

        // Forward ONLY to system clients - they decide what to do next
        let system_clients = self.connection_manager.get_system_clients();
        if system_clients.is_empty() {
            debug!("No system clients connected - message dropped");
            return Ok(());
        }

        self.connection_manager.send_to_system_clients(outgoing_message).await;
        
        info!(
            "RELAY[{}]: {} -> {} system clients",
            server_id,
            message.event_type,
            system_clients.len()
        );

        Ok(())
    }

    /// Route message from client - different behavior based on role
    pub async fn route_client_message(
        &self,
        connection_id: String,
        role: ClientRole,
        message: IncomingMessage,
    ) -> Result<()> {
        self.message_count.fetch_add(1, Ordering::Relaxed);
        
        // Handle acknowledgments first (replies to previous commands)
        if message.reply_to.is_some() {
            if let Some(_acknowledged_command) = self.command_tracker.handle_acknowledgment(&message) {
                // Command was acknowledged - could forward success to original requester here
                info!("Command acknowledged: {:?} -> {:?}", message.reply_to, message.event_type);
                return Ok(());
            }
        }
        
        info!(
            "CLIENT->SERVER: client={}, role={:?}, type={}, data={}",
            connection_id,
            role,
            message.event_type,
            serde_json::to_string(&message.data).unwrap_or_else(|_| "invalid_json".to_string())
        );

        // Handle authentication messages (these are processed at server level)
        if message.is_auth_message() {
            debug!("Auth message received from client {} - processed at server level", connection_id);
            return Ok(());
        }

        // Handle RCON commands directly for Admin and System clients
        if matches!(role, ClientRole::Admin | ClientRole::System) && message.event_type == "rcon_command" {
            self.handle_rcon_command_direct(connection_id, message).await?;
            return Ok(());
        }

        match role {
            ClientRole::System => {
                // System clients can send commands to adapters
                self.route_system_command(connection_id, message).await?;
            }
            ClientRole::Admin | ClientRole::Player | ClientRole::Guest => {
                // All other clients send messages to system clients for processing
                self.route_to_system_clients(connection_id, role, message).await?;
            }
        }

        Ok(())
    }

    /// Route system client commands to adapters
    async fn route_system_command(
        &self,
        system_client_id: String,
        message: IncomingMessage,
    ) -> Result<()> {
        info!(
            "SYSTEM->ADAPTER: system_client={}, type={}, data={}",
            system_client_id,
            message.event_type,
            serde_json::to_string(&message.data).unwrap_or_else(|_| "invalid_json".to_string())
        );

        // Extract target server from message data
        let target_server = message.data
            .get("server_id")
            .and_then(|s| s.as_str());

        let relay_message = RelayMessage::new(
            message.event_type.clone(),
            message.data.clone(),
            Some(system_client_id.clone()),
        );

        let mut outgoing_message = OutgoingMessage::new(
            message.event_type.clone(),
            serde_json::to_value(&relay_message)?,
        ).with_message_id(
            message.message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        );

        // Add acknowledgment tracking if the original message requests it
        if message.requires_ack == Some(true) {
            outgoing_message = outgoing_message
                .with_timeout(message.timeout_ms.unwrap_or(30000))
                .requires_acknowledgment();
        }

        if let Some(server_id) = target_server {
            // Track command if it requires acknowledgment
            if outgoing_message.requires_ack == Some(true) {
                self.command_tracker.track_command(&outgoing_message, system_client_id.clone());
            }
            
            // Send to specific adapter
            if !self.connection_manager.send_to_adapter(server_id, outgoing_message.clone()).await {
                warn!("No adapter found for server_id: {}", server_id);
                self.send_error_to_client(&system_client_id, &message.event_type, 
                    &format!("No adapter connected for server: {}", server_id)).await;
            } else {
                info!("RELAY: {} -> adapter[{}]", message.event_type, server_id);
            }
        } else {
            // Track command if it requires acknowledgment
            if outgoing_message.requires_ack == Some(true) {
                self.command_tracker.track_command(&outgoing_message, system_client_id.clone());
            }
            
            // Broadcast to all adapters
            let adapter_count = self.connection_manager.adapter_count();
            self.connection_manager.broadcast_to_adapters(outgoing_message).await;
            info!("RELAY: {} -> {} adapters", message.event_type, adapter_count);
        }

        Ok(())
    }

    /// Route non-system client messages to system clients for processing
    async fn route_to_system_clients(
        &self,
        client_id: String,
        role: ClientRole,
        message: IncomingMessage,
    ) -> Result<()> {
        info!(
            "CLIENT->SYSTEM: client={}, role={:?}, type={}, data={}",
            client_id,
            role,
            message.event_type,
            serde_json::to_string(&message.data).unwrap_or_else(|_| "invalid_json".to_string())
        );

        // Add client context to message data
        let mut enhanced_data = message.data.clone();
        if let serde_json::Value::Object(ref mut map) = enhanced_data {
            map.insert("client_id".to_string(), serde_json::Value::String(client_id.clone()));
            map.insert("client_role".to_string(), serde_json::Value::String(role.as_str().to_string()));
        }

        let relay_message = RelayMessage::new(
            message.event_type.clone(),
            enhanced_data,
            Some(client_id.clone()),
        );

        let outgoing_message = OutgoingMessage::new(
            message.event_type.clone(),
            serde_json::to_value(&relay_message)?,
        ).with_message_id(
            message.message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        );

        let system_clients = self.connection_manager.get_system_clients();
        if system_clients.is_empty() {
            warn!("No system clients connected - dropping client message");
            self.send_error_to_client(&client_id, &message.event_type, "No system clients available").await;
            return Ok(());
        }

        self.connection_manager.send_to_system_clients(outgoing_message).await;
        info!("RELAY: {} -> {} system clients", message.event_type, system_clients.len());

        Ok(())
    }

    /// Send error response to a specific client
    async fn send_error_to_client(&self, client_id: &str, original_event_type: &str, error_message: &str) {
        if let Some(client) = self.connection_manager.get_client(client_id) {
            let error_response = OutgoingMessage::error(
                format!("{}_error", original_event_type),
                error_message.to_string(),
            );
            
            if let Err(e) = client.message_sender.send(error_response) {
                error!("Failed to send error response to client {}: {}", client_id, e);
            } else {
                info!("ERROR->CLIENT: client={}, message={}", client_id, error_message);
            }
        }
    }

    /// Handle RCON registration for an adapter
    pub async fn register_adapter_rcon(&self, server_id: String, rcon_host: Option<String>, rcon_port: Option<u16>, rcon_password: Option<String>) -> Result<()> {
        // Only register RCON if password is provided
        if let Some(password) = rcon_password {
            if !password.is_empty() {
                let config = RconConfig {
                    host: rcon_host.unwrap_or_else(|| "127.0.0.1".to_string()),
                    port: rcon_port.unwrap_or(25575),
                    password,
                    timeout: Duration::from_secs(10),
                };
                
                info!("Registering RCON for server {} at {}:{}", server_id, config.host, config.port);
                self.rcon_manager.register_client(server_id.clone(), config).await?;
            } else {
                debug!("Empty RCON password for server {} - RCON disabled", server_id);
            }
        } else {
            debug!("No RCON configuration provided for server {} - RCON disabled", server_id);
        }
        
        Ok(())
    }

    /// Unregister RCON for an adapter
    pub async fn unregister_adapter_rcon(&self, server_id: &str) {
        info!("Unregistering RCON for server {}", server_id);
        self.rcon_manager.unregister_client(server_id).await;
    }

    /// Execute command via RCON if available, otherwise fall back to adapter
    pub async fn execute_command_with_rcon_fallback(&self, server_id: &str, command: &str, system_client_id: &str) -> Result<String> {
        // First, try RCON if available
        if self.rcon_manager.is_rcon_available(server_id).await {
            match self.rcon_manager.execute_command(server_id, command).await {
                Ok(result) => {
                    info!("RCON command executed for server {}: {} -> {}", server_id, command, result);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("RCON command failed for server {}, falling back to adapter: {}", server_id, e);
                }
            }
        }

        // Fallback to adapter command
        info!("Using adapter command for server {}: {}", server_id, command);
        
        // Create command message for adapter
        let command_message = IncomingMessage {
            event_type: "command".to_string(),
            data: serde_json::json!({
                "command": command,
                "server_id": server_id
            }),
            timestamp: Some(chrono::Utc::now().timestamp() as u64),
            message_id: Some(uuid::Uuid::new_v4().to_string()),
            reply_to: None,
            requires_ack: Some(true),
            timeout_ms: Some(30000),
        };

        // Route through system command handler
        self.route_system_command(system_client_id.to_string(), command_message).await?;
        
        Ok("Command sent to adapter (RCON unavailable)".to_string())
    }

    /// Handle RCON commands directly using the server's RCON integration
    async fn handle_rcon_command_direct(&self, client_id: String, message: IncomingMessage) -> Result<()> {
        info!(
            "RCON_DIRECT: client={}, command={}",
            client_id,
            message.data.get("command").and_then(|v| v.as_str()).unwrap_or("unknown")
        );

        let command = message.data.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing command in RCON request"))?;

        let server_id = message.data.get("server_id")
            .and_then(|v| v.as_str());

        // Use the RCON manager to execute the command
        let result = if let Some(server_id) = server_id {
            // Execute on specific server
            self.rcon_manager.execute_command(server_id, command).await
        } else {
            // For now, we need a server_id for RCON. Send error back.
            Err(anyhow::anyhow!("server_id is required for RCON commands"))
        };

        // Send response back to client
        let response_data = match result {
            Ok(output) => serde_json::json!({
                "success": true,
                "result": output,
                "command": command,
                "server_id": server_id,
                "via": "rcon_direct"
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "command": command,
                "server_id": server_id,
                "via": "rcon_direct"
            })
        };

        // Send response back to the client
        if let Some(message_id) = message.message_id {
            let response = OutgoingMessage::ack_success(message_id, response_data);
            self.connection_manager.send_to_client(&client_id, response).await;
        }

        info!("RCON command processed directly by server");
        Ok(())
    }

    pub fn get_message_count(&self) -> u64 {
        self.message_count.load(Ordering::Relaxed)
    }

    pub async fn get_routing_stats(&self) -> Result<RoutingStats> {
        let adapter_count = self.connection_manager.adapter_count();
        let client_count = self.connection_manager.client_count();
        let system_client_count = self.connection_manager.system_client_count();
        let total_messages = self.message_count.load(Ordering::Relaxed);

        Ok(RoutingStats {
            adapter_count,
            client_count,
            system_client_count: system_client_count as u64,
            total_messages_routed: total_messages,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RoutingStats {
    pub adapter_count: u64,
    pub client_count: u64,
    pub system_client_count: u64,
    pub total_messages_routed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionManager;

    #[tokio::test]
    async fn test_message_router_creation() {
        let connection_manager = Arc::new(ConnectionManager::new());
        let kv_store = Arc::new(KvStore::new());
        let command_tracker = Arc::new(CommandTracker::new());
        let rcon_manager = Arc::new(RconManager::new());
        let router = MessageRouter::new(connection_manager, kv_store, command_tracker, rcon_manager);
        
        let stats = router.get_routing_stats().await.unwrap();
        assert_eq!(stats.adapter_count, 0);
        assert_eq!(stats.client_count, 0);
        assert_eq!(stats.total_messages_routed, 0);
    }

    #[test]
    fn test_message_count_tracking() {
        let connection_manager = Arc::new(ConnectionManager::new());
        let kv_store = Arc::new(KvStore::new());
        let command_tracker = Arc::new(CommandTracker::new());
        let rcon_manager = Arc::new(RconManager::new());
        let router = MessageRouter::new(connection_manager, kv_store, command_tracker, rcon_manager);
        
        assert_eq!(router.get_message_count(), 0);
        
        // Simulate message processing
        router.message_count.fetch_add(5, Ordering::Relaxed);
        assert_eq!(router.get_message_count(), 5);
    }
}