use crate::auth::{AuthService, ValidatedAdapterToken, ValidatedClientToken};
use crate::connection::ConnectionManager;
use crate::message::IncomingMessage;
use crate::rate_limiter::{RateLimiter, RateLimitResult};
use crate::router::MessageRouter;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, WebSocketStream, MaybeTlsStream};
use tracing::{debug, error, info, warn};
use url::Url;

pub struct AdapterServer {
    port: u16,
    auth_service: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
    connection_manager: Arc<ConnectionManager>,
    message_router: Arc<MessageRouter>,
}

pub struct ClientServer {
    port: u16,
    auth_service: Arc<AuthService>,
    rate_limiter: Arc<RateLimiter>,
    connection_manager: Arc<ConnectionManager>,
    message_router: Arc<MessageRouter>,
}

impl AdapterServer {
    pub fn new(
        port: u16,
        auth_service: Arc<AuthService>,
        rate_limiter: Arc<RateLimiter>,
        connection_manager: Arc<ConnectionManager>,
        message_router: Arc<MessageRouter>,
    ) -> Self {
        Self {
            port,
            auth_service,
            rate_limiter,
            connection_manager,
            message_router,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("Adapter WebSocket server listening on {}", addr);

        while let Ok((stream, client_addr)) = listener.accept().await {
            let auth_service = Arc::clone(&self.auth_service);
            let rate_limiter = Arc::clone(&self.rate_limiter);
            let connection_manager = Arc::clone(&self.connection_manager);
            let message_router = Arc::clone(&self.message_router);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(
                    stream,
                    client_addr,
                    auth_service,
                    rate_limiter,
                    connection_manager,
                    message_router,
                ).await {
                    error!("Adapter connection error from {}: {}", client_addr, e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        auth_service: Arc<AuthService>,
        rate_limiter: Arc<RateLimiter>,
        connection_manager: Arc<ConnectionManager>,
        message_router: Arc<MessageRouter>,
    ) -> Result<()> {
        let client_ip = client_addr.ip().to_string();

        // First, check if IP is banned
        if rate_limiter.is_ip_banned(&client_ip).await? {
            warn!("Rejected connection from banned IP: {}", client_ip);
            return Ok(());
        }

        // Extract Authorization header during WebSocket handshake
        let auth_token = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let token_ref = auth_token.clone();
        
        let websocket = tokio_tungstenite::accept_hdr_async(stream, move |req: &tokio_tungstenite::tungstenite::handshake::server::Request, res: tokio_tungstenite::tungstenite::handshake::server::Response| {
            debug!("WebSocket handshake from {}", client_addr);
            
            // Extract Authorization header
            if let Some(auth_header) = req.headers().get("authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if auth_str.starts_with("Bearer ") {
                        *token_ref.lock().unwrap() = Some(auth_str[7..].to_string());
                        debug!("Authorization token extracted from header");
                    } else {
                        warn!("Invalid Authorization header format from {}", client_addr);
                    }
                } else {
                    warn!("Invalid Authorization header encoding from {}", client_addr);
                }
            } else {
                warn!("No Authorization header provided from {}", client_addr);
            }
            
            Ok(res)
        }).await?;
        
        debug!("WebSocket connection established from {}", client_addr);

        // Verify the extracted token
        let token = match auth_token.lock().unwrap().clone() {
            Some(token) => token,
            None => {
                error!("No valid Authorization token provided from {}", client_addr);
                return Ok(());
            }
        };
        
        let validated_token = match auth_service.verify_adapter_token(&token) {
            Ok(token) => {
                info!("Adapter authentication successful for server: {}", token.server_id);
                token
            }
            Err(e) => {
                error!("Authentication failed for {}: {}", client_addr, e);
                return Ok(());
            }
        };

        let connection_id = uuid::Uuid::new_v4().to_string();
        
        // Create message handler for this adapter
        let message_router_clone = Arc::clone(&message_router);
        let message_handler = move |server_id: String, message: crate::message::IncomingMessage| {
            let router = Arc::clone(&message_router_clone);
            async move {
                router.route_adapter_message(server_id, message).await
            }
        };
        
        // Add connection to manager
        match connection_manager.add_adapter_connection(
            connection_id.clone(),
            validated_token.clone(),
            websocket,
            message_handler,
        ).await {
            Ok(_connection) => {
                info!(
                    "Adapter {} connected from {} (server: {})",
                    connection_id, client_addr, validated_token.server_id
                );
            }
            Err(e) => {
                error!("Failed to add adapter connection {}: {}", connection_id, e);
            }
        }

        Ok(())
    }


}

impl ClientServer {
    pub fn new(
        port: u16,
        auth_service: Arc<AuthService>,
        rate_limiter: Arc<RateLimiter>,
        connection_manager: Arc<ConnectionManager>,
        message_router: Arc<MessageRouter>,
    ) -> Self {
        Self {
            port,
            auth_service,
            rate_limiter,
            connection_manager,
            message_router,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("Client WebSocket server listening on {}", addr);

        while let Ok((stream, client_addr)) = listener.accept().await {
            let auth_service = Arc::clone(&self.auth_service);
            let rate_limiter = Arc::clone(&self.rate_limiter);
            let connection_manager = Arc::clone(&self.connection_manager);
            let message_router = Arc::clone(&self.message_router);

            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(
                    stream,
                    client_addr,
                    auth_service,
                    rate_limiter,
                    connection_manager,
                    message_router,
                ).await {
                    error!("Client connection error from {}: {}", client_addr, e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(
        stream: TcpStream,
        client_addr: SocketAddr,
        auth_service: Arc<AuthService>,
        rate_limiter: Arc<RateLimiter>,
        connection_manager: Arc<ConnectionManager>,
        message_router: Arc<MessageRouter>,
    ) -> Result<()> {
        let client_ip = client_addr.ip().to_string();

        // Check if IP is banned
        if rate_limiter.is_ip_banned(&client_ip).await? {
            warn!("Rejected connection from banned IP: {}", client_ip);
            return Ok(());
        }

        // Accept WebSocket connection
        let mut websocket = accept_async(stream).await?;
        
        debug!("Client WebSocket connection established from {}", client_addr);

        let connection_id = uuid::Uuid::new_v4().to_string();
        let mut authenticated_token: Option<ValidatedClientToken> = None;

        // Send initial connection acknowledgment
        let ack_message = serde_json::json!({
            "type": "connection_established",
            "socketId": connection_id,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
        
        websocket.send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&ack_message)?
        )).await?;

        // Wait for messages (auth or regular messages)
        while let Some(msg_result) = websocket.next().await {
            match msg_result {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    match serde_json::from_str::<IncomingMessage>(&text) {
                        Ok(message) => {
                            // Handle authentication messages
                            if message.is_auth_message() && authenticated_token.is_none() {
                                authenticated_token = Self::handle_auth_message(
                                    &message,
                                    &auth_service,
                                    &rate_limiter,
                                    &client_ip,
                                    &connection_id,
                                    &mut websocket,
                                ).await?;
                                
                                // After successful auth, add connection to manager
                                if authenticated_token.is_some() {
                                    match connection_manager.add_client_connection(
                                        connection_id.clone(),
                                        authenticated_token.clone(),
                                        websocket,
                                    ).await {
                                        Ok(_) => {
                                            info!("Client {} connected from {}", connection_id, client_addr);
                                            return Ok(()); // Connection is now managed by ConnectionManager
                                        }
                                        Err(e) => {
                                            error!("Failed to add client connection {}: {}", connection_id, e);
                                            return Ok(());
                                        }
                                    }
                                }
                            } else {
                                // Handle regular messages (for guests or pre-auth)
                                let role = authenticated_token.as_ref()
                                    .map(|t| t.role.clone())
                                    .unwrap_or(crate::auth::ClientRole::Guest);
                                
                                // Check rate limit
                                let rate_result = rate_limiter.check_client_rate_limit(
                                    &client_ip,
                                    &role,
                                    &message.event_type,
                                ).await?;

                                if !rate_result.is_allowed() {
                                    let error_msg = serde_json::json!({
                                        "type": "error",
                                        "message": rate_result.to_error_message().unwrap_or("Rate limit exceeded".to_string()),
                                        "socketId": connection_id
                                    });
                                    
                                    websocket.send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::to_string(&error_msg)?
                                    )).await?;
                                    continue;
                                }

                                // Route message
                                if let Err(e) = message_router.route_client_message(
                                    connection_id.clone(),
                                    role,
                                    message,
                                ).await {
                                    error!("Failed to route client message: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Invalid JSON from client {}: {}", client_addr, e);
                        }
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    debug!("Client {} closed connection", client_addr);
                    break;
                }
                Ok(_) => {
                    // Handle other message types (binary, ping, pong) if needed
                }
                Err(e) => {
                    error!("WebSocket error from client {}: {}", client_addr, e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_auth_message(
        message: &IncomingMessage,
        auth_service: &AuthService,
        rate_limiter: &RateLimiter,
        client_ip: &str,
        connection_id: &str,
        websocket: &mut WebSocketStream<TcpStream>,
    ) -> Result<Option<ValidatedClientToken>> {
        debug!("Processing client auth message");

        // Check if we can parse auth data from message
        if let Ok(auth_data) = serde_json::from_value::<serde_json::Value>(message.data.clone()) {
            if let Some(token_str) = auth_data.get("token").and_then(|t| t.as_str()) {
                // Validate client token
                match auth_service.verify_client_token(token_str) {
                    Ok(validated_token) => {
                        info!("Client authentication successful for role: {:?}", validated_token.role);

                        let success_msg = serde_json::json!({
                            "type": "authenticated",
                            "success": true,
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            "data": {
                                "socketId": connection_id,
                                "connectionId": connection_id,
                                "role": format!("{:?}", validated_token.role).to_lowercase(),
                                "user": {
                                    "username": validated_token.username.clone().unwrap_or("SystemClient".to_string()),
                                    "role": format!("{:?}", validated_token.role).to_lowercase()
                                },
                                "server": {
                                    "name": "Bcon Server",
                                    "version": "1.0.0",
                                    "authenticated": true
                                }
                            }
                        });

                        websocket.send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::to_string(&success_msg)?
                        )).await?;

                        return Ok(Some(validated_token));
                    }
                    Err(e) => {
                        warn!("Client authentication failed: {}", e);
                        rate_limiter.record_failed_auth(client_ip).await?;

                        let error_msg = serde_json::json!({
                            "type": "auth_failed", 
                            "success": false,
                            "timestamp": std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            "data": {
                                "socketId": connection_id,
                                "message": format!("Authentication failed: {}", e)
                            }
                        });

                        websocket.send(tokio_tungstenite::tungstenite::Message::Text(
                            serde_json::to_string(&error_msg)?
                        )).await?;
                    }
                }
            }
        }

        // Auth failed - send error
        let error_msg = serde_json::json!({
            "type": "auth_failed",
            "success": false,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "data": {
                "socketId": connection_id,
                "message": "Invalid authentication data"
            }
        });

        websocket.send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::to_string(&error_msg)?
        )).await?;

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BconConfig;

    #[tokio::test]
    async fn test_server_creation() {
        let config = BconConfig::default();
        let auth_service = Arc::new(AuthService::new(
            "test_adapter_secret".to_string(),
            "test_client_secret".to_string(),
        ).unwrap());
        let kv_store = Arc::new(crate::kv_store::KvStore::new());
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limits.clone(), kv_store.clone()));
        let connection_manager = Arc::new(ConnectionManager::new());
        let message_router = Arc::new(MessageRouter::new(connection_manager.clone(), kv_store));

        let adapter_server = AdapterServer::new(
            8082,
            auth_service.clone(),
            rate_limiter.clone(),
            connection_manager.clone(),
            message_router.clone(),
        );

        let client_server = ClientServer::new(
            8081,
            auth_service,
            rate_limiter,
            connection_manager,
            message_router,
        );

        // Just test that servers can be created without panicking
        assert_eq!(adapter_server.port, 8082);
        assert_eq!(client_server.port, 8081);
    }
}