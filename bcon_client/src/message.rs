use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message received from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
}

/// Message to send to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: serde_json::Value,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    pub timestamp: Option<u64>,
}

/// Relay message received from adapters (via system clients)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

impl IncomingMessage {
    /// Check if this is an authentication response
    pub fn is_auth_response(&self) -> bool {
        self.message_type == "authenticated" || self.message_type == "auth_failed"
    }
    
    /// Check if this is a successful response
    pub fn is_success(&self) -> bool {
        self.success.unwrap_or(true) && self.error.is_none()
    }
    
    /// Check if this is an error response
    pub fn is_error(&self) -> bool {
        !self.is_success()
    }
    
    /// Get error message if this is an error
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    
    /// Extract relay message if this contains one
    pub fn extract_relay_message(&self) -> Result<RelayMessage, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
    
    /// Check if this message came from an adapter (contains relay data)
    pub fn is_from_adapter(&self) -> bool {
        self.extract_relay_message().is_ok()
    }
}

impl OutgoingMessage {
    /// Create a new outgoing message
    pub fn new(event_type: String, data: serde_json::Value) -> Self {
        Self {
            event_type,
            data,
            message_id: Some(Uuid::new_v4().to_string()),
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
        }
    }
    
    /// Create a message with a specific ID
    pub fn with_id(mut self, id: String) -> Self {
        self.message_id = Some(id);
        self
    }
    
    /// Create a heartbeat/ping message
    pub fn heartbeat() -> Self {
        Self::new("heartbeat".to_string(), serde_json::json!({}))
    }
    
    /// Create a command message for system clients to send to adapters
    pub fn adapter_command(server_id: Option<String>, command_type: String, data: serde_json::Value) -> Self {
        let mut payload = data;
        if let Some(id) = server_id {
            payload["server_id"] = serde_json::Value::String(id);
        }
        
        Self::new(command_type, payload)
    }
    
    /// Create a chat message for players/admins
    pub fn chat_message(message: String, server_id: Option<String>) -> Self {
        let mut data = serde_json::json!({ "message": message });
        if let Some(id) = server_id {
            data["server_id"] = serde_json::Value::String(id);
        }
        
        Self::new("send_chat".to_string(), data)
    }
    
    /// Create a command execution message for admins
    pub fn execute_command(command: String, server_id: Option<String>) -> Self {
        let mut data = serde_json::json!({ "command": command });
        if let Some(id) = server_id {
            data["server_id"] = serde_json::Value::String(id);
        }
        
        Self::new("execute_command".to_string(), data)
    }
    
    /// Request server information
    pub fn get_server_info() -> Self {
        Self::new("get_server_info".to_string(), serde_json::json!({}))
    }
}

/// Event types commonly received from adapters
pub mod events {
    use super::*;
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlayerEvent {
        #[serde(rename = "playerId")]
        pub player_id: String,
        #[serde(rename = "playerName")]
        pub player_name: String,
        pub x: f64,
        pub y: f64,
        pub z: f64,
        pub dimension: String,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ChatEvent {
        pub message: String,
        pub sender: String,
        pub timestamp: u64,
        pub channel: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CommandEvent {
        pub command: String,
        pub sender: String,
        #[serde(rename = "senderType")]
        pub sender_type: String,
        pub arguments: serde_json::Value,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServerStatusEvent {
        pub status: String, // "started", "stopped", "restarting"
        #[serde(rename = "playerCount")]
        pub player_count: Option<u32>,
        pub uptime: Option<u64>,
    }
}

/// Message builder for common message types
pub struct MessageBuilder;

impl MessageBuilder {
    /// Build authentication message
    pub fn auth(auth_data: crate::auth::AuthData) -> OutgoingMessage {
        OutgoingMessage::new(
            "auth".to_string(),
            serde_json::to_value(auth_data).unwrap_or(serde_json::Value::Null)
        )
    }
    
    /// Build system client command to adapter
    pub fn system_command(command_type: String, server_id: Option<String>, data: serde_json::Value) -> OutgoingMessage {
        OutgoingMessage::adapter_command(server_id, command_type, data)
    }
    
    /// Build player chat message
    pub fn player_chat(message: String, server_id: Option<String>) -> OutgoingMessage {
        OutgoingMessage::chat_message(message, server_id)
    }
    
    /// Build admin command
    pub fn admin_command(command: String, server_id: Option<String>) -> OutgoingMessage {
        OutgoingMessage::execute_command(command, server_id)
    }
    
    /// Build custom event
    pub fn custom_event(event_type: String, data: serde_json::Value) -> OutgoingMessage {
        OutgoingMessage::new(event_type, data)
    }
}

#[cfg(feature = "native")]
/// Response handler for tracking request/response pairs
pub struct ResponseTracker {
    pending_requests: std::collections::HashMap<String, tokio::sync::oneshot::Sender<IncomingMessage>>,
}

#[cfg(feature = "native")]
impl ResponseTracker {
    pub fn new() -> Self {
        Self {
            pending_requests: std::collections::HashMap::new(),
        }
    }
    
    /// Add a pending request
    pub fn add_request(&mut self, message_id: String, sender: tokio::sync::oneshot::Sender<IncomingMessage>) {
        self.pending_requests.insert(message_id, sender);
    }
    
    /// Handle incoming response
    pub fn handle_response(&mut self, message: &IncomingMessage) -> bool {
        if let Some(message_id) = &message.message_id {
            if let Some(sender) = self.pending_requests.remove(message_id) {
                let _ = sender.send(message.clone());
                return true;
            }
        }
        false
    }
    
    /// Clean up expired requests
    pub fn cleanup_expired(&mut self) {
        // In a real implementation, you'd track timestamps and remove expired requests
        // For now, we'll keep this simple
    }
}

#[cfg(feature = "native")]
impl Default for ResponseTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outgoing_message_creation() {
        let msg = OutgoingMessage::new(
            "test_event".to_string(),
            serde_json::json!({"key": "value"})
        );
        
        assert_eq!(msg.event_type, "test_event");
        assert!(msg.message_id.is_some());
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_incoming_message_auth_detection() {
        let auth_success = IncomingMessage {
            message_type: "authenticated".to_string(),
            data: serde_json::Value::Null,
            timestamp: 0,
            success: Some(true),
            error: None,
            message_id: None,
        };
        
        assert!(auth_success.is_auth_response());
        assert!(auth_success.is_success());
        
        let auth_fail = IncomingMessage {
            message_type: "auth_failed".to_string(),
            data: serde_json::Value::Null,
            timestamp: 0,
            success: Some(false),
            error: Some("Invalid credentials".to_string()),
            message_id: None,
        };
        
        assert!(auth_fail.is_auth_response());
        assert!(auth_fail.is_error());
        assert_eq!(auth_fail.get_error(), Some("Invalid credentials"));
    }

    #[test]
    fn test_message_builder() {
        let chat_msg = MessageBuilder::player_chat(
            "Hello world!".to_string(),
            Some("test_server".to_string())
        );
        
        assert_eq!(chat_msg.event_type, "send_chat");
        assert!(chat_msg.data.get("message").is_some());
        assert!(chat_msg.data.get("server_id").is_some());
        
        let heartbeat = OutgoingMessage::heartbeat();
        assert_eq!(heartbeat.event_type, "heartbeat");
    }
}