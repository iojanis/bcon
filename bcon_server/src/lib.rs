pub mod auth;
pub mod command_tracker;
pub mod config;
pub mod connection;
pub mod error;
pub mod kv_store;
pub mod message;
pub mod rate_limiter;
pub mod rcon_client;
pub mod router;
pub mod server;

pub use auth::*;
pub use command_tracker::*;
pub use config::*;
pub use connection::*;
pub use error::*;
pub use kv_store::*;
pub use message::*;
pub use rate_limiter::*;
pub use rcon_client::*;
pub use router::*;
pub use server::*;

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

pub struct BconServer {
    config: BconConfig,
    auth_service: Arc<AuthService>,
    kv_store: Arc<KvStore>,
    rate_limiter: Arc<RateLimiter>,
    connection_manager: Arc<ConnectionManager>,
    message_router: Arc<MessageRouter>,
    command_tracker: Arc<CommandTracker>,
    rcon_manager: Arc<RconManager>,
}

impl BconServer {
    pub fn new(config: BconConfig) -> Result<Self> {
        let kv_store = Arc::new(KvStore::new());
        let auth_service = Arc::new(AuthService::new(
            config.adapter_secret.clone(),
            config.client_secret.clone(),
        )?);
        
        let rate_limiter = Arc::new(RateLimiter::new(
            config.rate_limits.clone(),
            Arc::clone(&kv_store),
        ));
        
        let connection_manager = Arc::new(ConnectionManager::new());
        
        let mut command_tracker = CommandTracker::new();
        command_tracker.start_timeout_checker();
        let command_tracker = Arc::new(command_tracker);

        let rcon_manager = Arc::new(RconManager::new());
        
        let message_router = Arc::new(MessageRouter::new(
            Arc::clone(&connection_manager),
            Arc::clone(&kv_store),
            Arc::clone(&command_tracker),
            Arc::clone(&rcon_manager),
        ));

        Ok(Self {
            config,
            auth_service,
            kv_store,
            rate_limiter,
            connection_manager,
            message_router,
            command_tracker,
            rcon_manager,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting Bcon server...");
        
        let adapter_server = AdapterServer::new(
            self.config.adapter_port,
            Arc::clone(&self.auth_service),
            Arc::clone(&self.rate_limiter),
            Arc::clone(&self.connection_manager),
            Arc::clone(&self.message_router),
        );

        let client_server = ClientServer::new(
            self.config.client_port,
            Arc::clone(&self.auth_service),
            Arc::clone(&self.rate_limiter),
            Arc::clone(&self.connection_manager),
            Arc::clone(&self.message_router),
        );

        tokio::try_join!(
            adapter_server.start(),
            client_server.start(),
        )?;

        Ok(())
    }

    pub fn get_kv_store(&self) -> Arc<KvStore> {
        Arc::clone(&self.kv_store)
    }

    pub fn get_metrics(&self) -> BconMetrics {
        BconMetrics {
            active_adapters: self.connection_manager.adapter_count(),
            active_clients: self.connection_manager.client_count(),
            messages_per_second: self.message_router.get_message_count() as f64, // Total message count (rate calculation removed)
            connection_errors: self.rate_limiter.get_error_count(),
            authentication_failures: self.auth_service.get_failure_count(),
        }
    }

    /// Get RCON manager for handling RCON connections
    pub fn get_rcon_manager(&self) -> Arc<RconManager> {
        Arc::clone(&self.rcon_manager)
    }

    /// Get message router for RCON integration
    pub fn get_message_router(&self) -> Arc<MessageRouter> {
        Arc::clone(&self.message_router)
    }
}

#[derive(Debug, Clone)]
pub struct BconMetrics {
    pub active_adapters: u64,
    pub active_clients: u64,
    pub messages_per_second: f64,
    pub connection_errors: u64,
    pub authentication_failures: u64,
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub struct WasmBconServer {
        inner: BconServer,
    }

    #[wasm_bindgen]
    impl WasmBconServer {
        #[wasm_bindgen(constructor)]
        pub fn new(config_json: &str) -> Result<WasmBconServer, JsValue> {
            let config: BconConfig = serde_json::from_str(config_json)
                .map_err(|e| JsValue::from_str(&format!("Config parse error: {}", e)))?;
            
            let server = BconServer::new(config)
                .map_err(|e| JsValue::from_str(&format!("Server creation error: {}", e)))?;
            
            Ok(WasmBconServer { inner: server })
        }

        #[wasm_bindgen]
        pub async fn start(&self) -> Result<(), JsValue> {
            self.inner.start().await
                .map_err(|e| JsValue::from_str(&format!("Server start error: {}", e)))
        }

        #[wasm_bindgen]
        pub fn get_metrics(&self) -> JsValue {
            let metrics = self.inner.get_metrics();
            serde_wasm_bindgen::to_value(&metrics).unwrap_or(JsValue::NULL)
        }

        #[wasm_bindgen]
        pub fn kv_set(&self, key: &str, value: &str) -> Result<(), JsValue> {
            self.inner.get_kv_store().set(key, value.into())
                .map_err(|e| JsValue::from_str(&format!("KV set error: {}", e)))
        }

        #[wasm_bindgen]
        pub fn kv_get(&self, key: &str) -> JsValue {
            match self.inner.get_kv_store().get(key) {
                Ok(Some(value)) => serde_wasm_bindgen::to_value(&value).unwrap_or(JsValue::NULL),
                _ => JsValue::NULL,
            }
        }

        #[wasm_bindgen]
        pub fn kv_delete(&self, key: &str) -> Result<(), JsValue> {
            self.inner.get_kv_store().delete(key)
                .map_err(|e| JsValue::from_str(&format!("KV delete error: {}", e)))
        }
    }
}