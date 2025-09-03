//! Bcon Client Library
//!
//! A WebSocket client for connecting to Bcon servers with support for all client roles.
//! Works natively, in Node.js/Deno, and in web browsers via WASM.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod client;
pub mod auth;
pub mod message;

#[cfg(feature = "native")]
pub mod native;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub use wasm::{WasmBconClient, WasmBconClientBuilder};

// Re-export main types
#[cfg(feature = "native")]
pub use client::BconClient;
pub use auth::{ClientRole, AuthConfig};
pub use message::*;

#[derive(Error, Debug, Clone)]
pub enum BconError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Message parsing error: {0}")]
    MessageParsing(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Operation timeout")]
    Timeout,

    #[error("Client not connected")]
    NotConnected,

    #[error("Permission denied for role {role:?}")]
    PermissionDenied { role: ClientRole },
}

pub type Result<T> = std::result::Result<T, BconError>;

impl From<serde_json::Error> for BconError {
    fn from(err: serde_json::Error) -> Self {
        BconError::MessageParsing(err.to_string())
    }
}

/// Configuration for connecting to a Bcon server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BconConfig {
    /// Server URL (ws://localhost:8081 for client connections)
    pub server_url: String,

    /// Authentication configuration
    pub auth: Option<AuthConfig>,

    /// Connection timeout in milliseconds
    pub connect_timeout: u64,

    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,

    /// Maximum reconnection attempts (0 = infinite)
    pub max_reconnect_attempts: u32,

    /// Reconnection delay in milliseconds
    pub reconnection_delay: u64,
}

impl Default for BconConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://localhost:8081".to_string(),
            auth: None,
            connect_timeout: 30000,
            heartbeat_interval: 30000,
            max_reconnect_attempts: 5,
            reconnection_delay: 5000,
        }
    }
}

impl BconConfig {
    /// Create config for a guest connection (no authentication)
    pub fn guest(server_url: String) -> Self {
        Self {
            server_url,
            auth: None,
            ..Default::default()
        }
    }

    /// Create config for an authenticated connection
    pub fn authenticated(server_url: String, auth: AuthConfig) -> Self {
        Self {
            server_url,
            auth: Some(auth),
            ..Default::default()
        }
    }

    /// Create config for a system client connection
    pub fn system(server_url: String, token: String) -> Self {
        Self {
            server_url,
            auth: Some(AuthConfig::system(token)),
            ..Default::default()
        }
    }
}

/// Event handler trait for receiving messages from the server
pub trait BconEventHandler: Send + Sync {
    /// Called when successfully connected and authenticated
    fn on_connected(&mut self, client_info: ClientInfo);

    /// Called when disconnected from server
    fn on_disconnected(&mut self, reason: String);

    /// Called when a message is received from the server
    fn on_message(&mut self, message: IncomingMessage);

    /// Called when an error occurs
    fn on_error(&mut self, error: BconError);

    /// Called when authentication fails
    fn on_auth_failed(&mut self, reason: String);
}

/// Information about the connected client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub connection_id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub role: ClientRole,
    pub server_info: Option<ServerInfo>,
}

/// Server information received after connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Message statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageStats {
    pub sent: u64,
    pub received: u64,
    pub errors: u64,
    pub reconnections: u64,
}

// WASM exports
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
impl From<BconError> for wasm_bindgen::JsValue {
    fn from(error: BconError) -> Self {
        wasm_bindgen::JsValue::from_str(&error.to_string())
    }
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

