use crate::auth::{ClientRole, ValidatedAdapterToken, ValidatedClientToken};
use crate::message::{IncomingMessage, OutgoingMessage};
use dashmap::DashMap;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tracing::{debug, error, info, warn};

pub type WebSocket = WebSocketStream<tokio::net::TcpStream>;

#[derive(Debug, Clone)]
pub struct AdapterConnection {
    pub connection_id: String,
    pub server_id: String,
    pub server_name: Option<String>,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    pub message_sender: mpsc::UnboundedSender<OutgoingMessage>,
}

#[derive(Debug, Clone)]
pub struct ClientConnection {
    pub connection_id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub role: ClientRole,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub message_sender: mpsc::UnboundedSender<OutgoingMessage>,
}

pub struct ConnectionManager {
    adapters: DashMap<String, AdapterConnection>,
    clients: DashMap<String, ClientConnection>,
    system_clients: DashMap<String, Weak<ClientConnection>>,
    adapter_count: Arc<AtomicU64>,
    client_count: Arc<AtomicU64>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            adapters: DashMap::new(),
            clients: DashMap::new(),
            system_clients: DashMap::new(),
            adapter_count: Arc::new(AtomicU64::new(0)),
            client_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn add_adapter_connection<F, Fut>(
        &self,
        connection_id: String,
        validated_token: ValidatedAdapterToken,
        websocket: WebSocket,
        message_handler: F,
    ) -> Result<Arc<AdapterConnection>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(String, IncomingMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
    {
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        
        let connection = Arc::new(AdapterConnection {
            connection_id: connection_id.clone(),
            server_id: validated_token.server_id.clone(),
            server_name: validated_token.server_name,
            connected_at: Instant::now(),
            last_heartbeat: Instant::now(),
            message_sender,
        });

        // Insert connection
        self.adapters.insert(connection_id.clone(), (*connection).clone());
        self.adapter_count.fetch_add(1, Ordering::Relaxed);

        info!(
            "Adapter connected: {}",
            validated_token.server_id
        );

        // Spawn WebSocket handler
        let connection_clone = Arc::clone(&connection);
        let adapters_clone = self.adapters.clone();
        let adapter_count_clone = Arc::clone(&self.adapter_count);
        
        tokio::spawn(async move {
            if let Err(e) = Self::handle_adapter_websocket(
                websocket,
                connection_clone,
                &mut message_receiver,
                message_handler,
            ).await {
                error!("Adapter WebSocket error: {}", e);
            }

            // Cleanup on disconnection
            adapters_clone.remove(&connection_id);
            adapter_count_clone.fetch_sub(1, Ordering::Relaxed);
            info!("Adapter disconnected: {}", connection_id);
        });

        Ok(connection)
    }

    pub async fn add_client_connection<F, Fut>(
        &self,
        connection_id: String,
        validated_token: Option<ValidatedClientToken>,
        websocket: WebSocket,
        message_handler: F,
    ) -> Result<Arc<ClientConnection>, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(String, ClientRole, IncomingMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
    {
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        
        let (user_id, username, role) = if let Some(token) = validated_token {
            (token.user_id, token.username, token.role)
        } else {
            (None, None, ClientRole::Guest)
        };

        let connection = Arc::new(ClientConnection {
            connection_id: connection_id.clone(),
            user_id,
            username: username.clone(),
            role: role.clone(),
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            message_sender,
        });

        // Insert connection
        self.clients.insert(connection_id.clone(), (*connection).clone());
        self.client_count.fetch_add(1, Ordering::Relaxed);

        // Track system clients separately for efficient routing
        if role == ClientRole::System {
            self.system_clients.insert(connection_id.clone(), Arc::downgrade(&connection));
        }

        info!(
            "Client connected: {} as {:?}",
            username.as_deref().unwrap_or("guest"), role
        );

        // Spawn WebSocket handler
        let connection_clone = Arc::clone(&connection);
        let clients_clone = self.clients.clone();
        let system_clients_clone = self.system_clients.clone();
        let client_count_clone = Arc::clone(&self.client_count);
        
        tokio::spawn(async move {
            if let Err(e) = Self::handle_client_websocket(
                websocket,
                connection_clone,
                &mut message_receiver,
                message_handler,
            ).await {
                error!("Client WebSocket error: {}", e);
            }

            // Cleanup on disconnection
            clients_clone.remove(&connection_id);
            if role == ClientRole::System {
                system_clients_clone.remove(&connection_id);
            }
            client_count_clone.fetch_sub(1, Ordering::Relaxed);
            info!("Client disconnected: {} ({:?})", connection_id, username);
        });

        Ok(connection)
    }

    async fn handle_adapter_websocket<F, Fut>(
        mut websocket: WebSocket,
        connection: Arc<AdapterConnection>,
        message_receiver: &mut mpsc::UnboundedReceiver<OutgoingMessage>,
        message_handler: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(String, IncomingMessage) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
    {
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                ws_msg = websocket.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                                // Parse incoming message
                                match serde_json::from_str::<IncomingMessage>(&text) {
                                    Ok(incoming_message) => {
                                        // Route message through the handler
                                        if let Err(e) = message_handler(connection.server_id.clone(), incoming_message).await {
                                            error!("Failed to route adapter message: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Invalid JSON from adapter {} (server: {}): {} - Raw: {}", 
                                            connection.connection_id, connection.server_id, e, text);
                                    }
                                }
                            } else if let tokio_tungstenite::tungstenite::Message::Close(_) = msg {
                                info!("Adapter {} (server: {}) closed connection", connection.connection_id, connection.server_id);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error for adapter {} (server: {}): {}", connection.connection_id, connection.server_id, e);
                            break;
                        }
                        None => {
                            info!("Adapter {} (server: {}) connection closed", connection.connection_id, connection.server_id);
                            break;
                        }
                    }
                }
                
                // Handle outgoing messages
                outgoing_msg = message_receiver.recv() => {
                    match outgoing_msg {
                        Some(msg) => {
                            let json = serde_json::to_string(&msg)?;
                            if let Err(e) = websocket.send(
                                tokio_tungstenite::tungstenite::Message::Text(json)
                            ).await {
                                error!("Failed to send message to adapter {}: {}", connection.connection_id, e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_client_websocket<F, Fut>(
        mut websocket: WebSocket,
        connection: Arc<ClientConnection>,
        message_receiver: &mut mpsc::UnboundedReceiver<OutgoingMessage>,
        message_handler: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn(String, ClientRole, IncomingMessage) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<(), anyhow::Error>> + Send,
    {
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                ws_msg = websocket.next() => {
                    match ws_msg {
                        Some(Ok(msg)) => {
                            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                                debug!("Client {} received: {}", connection.connection_id, text);
                                
                                // Parse incoming message and route it
                                match serde_json::from_str::<IncomingMessage>(&text) {
                                    Ok(incoming_message) => {
                                        // Route message through the handler
                                        if let Err(e) = message_handler(connection.connection_id.clone(), connection.role.clone(), incoming_message).await {
                                            error!("Failed to route client message: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Invalid JSON from client {} (role: {:?}): {} - Raw: {}", 
                                            connection.connection_id, connection.role, e, text);
                                    }
                                }
                            } else if let tokio_tungstenite::tungstenite::Message::Close(_) = msg {
                                info!("Client {} ({:?}) closed connection", connection.connection_id, connection.username);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error for client {}: {}", connection.connection_id, e);
                            break;
                        }
                        None => break,
                    }
                }
                
                // Handle outgoing messages
                outgoing_msg = message_receiver.recv() => {
                    match outgoing_msg {
                        Some(msg) => {
                            let json = serde_json::to_string(&msg)?;
                            if let Err(e) = websocket.send(
                                tokio_tungstenite::tungstenite::Message::Text(json)
                            ).await {
                                error!("Failed to send message to client {}: {}", connection.connection_id, e);
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_adapter(&self, connection_id: &str) -> Option<AdapterConnection> {
        self.adapters.get(connection_id).map(|entry| entry.clone())
    }

    pub fn get_client(&self, connection_id: &str) -> Option<ClientConnection> {
        self.clients.get(connection_id).map(|entry| entry.clone())
    }

    pub fn get_adapters_by_server(&self, server_id: &str) -> Vec<AdapterConnection> {
        self.adapters
            .iter()
            .filter(|entry| entry.server_id == server_id)
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn get_system_clients(&self) -> Vec<ClientConnection> {
        let mut active_clients = Vec::new();
        let mut expired_keys = Vec::new();

        for entry in self.system_clients.iter() {
            if let Some(connection) = entry.value().upgrade() {
                if let Some(client) = self.clients.get(&connection.connection_id) {
                    active_clients.push(client.clone());
                }
            } else {
                expired_keys.push(entry.key().clone());
            }
        }

        // Clean up expired weak references
        for key in expired_keys {
            self.system_clients.remove(&key);
        }

        active_clients
    }

    pub fn get_clients_by_role(&self, role: ClientRole) -> Vec<ClientConnection> {
        self.clients
            .iter()
            .filter(|entry| entry.role == role)
            .map(|entry| entry.clone())
            .collect()
    }

    pub fn get_all_clients(&self) -> Vec<ClientConnection> {
        self.clients.iter().map(|entry| entry.clone()).collect()
    }

    pub fn get_all_adapters(&self) -> Vec<AdapterConnection> {
        self.adapters.iter().map(|entry| entry.clone()).collect()
    }

    pub fn adapter_count(&self) -> u64 {
        self.adapter_count.load(Ordering::Relaxed)
    }

    pub fn client_count(&self) -> u64 {
        self.client_count.load(Ordering::Relaxed)
    }

    pub fn system_client_count(&self) -> usize {
        self.get_system_clients().len()
    }

    pub async fn broadcast_to_adapters(&self, message: OutgoingMessage) {
        for adapter in self.adapters.iter() {
            if let Err(e) = adapter.message_sender.send(message.clone()) {
                warn!("Failed to send message to adapter {}: {}", adapter.connection_id, e);
            }
        }
    }

    pub async fn broadcast_to_clients(&self, message: OutgoingMessage, role_filter: Option<ClientRole>) {
        for client in self.clients.iter() {
            if let Some(filter) = &role_filter {
                if &client.role != filter {
                    continue;
                }
            }
            
            if let Err(e) = client.message_sender.send(message.clone()) {
                warn!("Failed to send message to client {}: {}", client.connection_id, e);
            }
        }
    }

    pub async fn send_to_adapter(&self, server_id: &str, message: OutgoingMessage) -> bool {
        let adapters = self.get_adapters_by_server(server_id);
        if adapters.is_empty() {
            warn!("No adapters found for server_id: {}", server_id);
            return false;
        }

        let mut sent = false;
        for adapter in adapters {
            if let Err(e) = adapter.message_sender.send(message.clone()) {
                warn!("Failed to send message to adapter {} ({}): {}", adapter.connection_id, server_id, e);
            } else {
                sent = true;
            }
        }
        
        sent
    }

    pub async fn send_to_system_clients(&self, message: OutgoingMessage) {
        for client in self.get_system_clients() {
            if let Err(e) = client.message_sender.send(message.clone()) {
                warn!("Failed to send message to system client {}: {}", client.connection_id, e);
            }
        }
    }

    pub async fn send_to_client(&self, client_id: &str, message: OutgoingMessage) {
        if let Some(client) = self.get_client(client_id) {
            if let Err(e) = client.message_sender.send(message) {
                warn!("Failed to send message to client {}: {}", client_id, e);
            }
        } else {
            warn!("Client not found for ID: {}", client_id);
        }
    }

    pub fn remove_adapter(&self, connection_id: &str) {
        if self.adapters.remove(connection_id).is_some() {
            self.adapter_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    pub fn remove_client(&self, connection_id: &str) {
        if let Some((_, client)) = self.clients.remove(connection_id) {
            self.client_count.fetch_sub(1, Ordering::Relaxed);
            if client.role == ClientRole::System {
                self.system_clients.remove(connection_id);
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ValidatedAdapterToken;

    #[test]
    fn test_connection_manager_creation() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.adapter_count(), 0);
        assert_eq!(manager.client_count(), 0);
    }

    #[test]
    fn test_client_role_filtering() {
        let manager = ConnectionManager::new();
        
        // This test would need actual connections to be meaningful
        // In a real test, you'd create mock connections and test filtering
        assert_eq!(manager.get_clients_by_role(ClientRole::System).len(), 0);
    }
}