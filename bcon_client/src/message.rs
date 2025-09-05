use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message received from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub data: serde_json::Value,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "replyTo")]
    pub reply_to: Option<String>,
}

/// Message to send to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: serde_json::Value,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(rename = "replyTo")]
    pub reply_to: Option<String>,
    pub timestamp: Option<u64>,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    #[serde(rename = "requiresAck")]
    pub requires_ack: Option<bool>,
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
    
    /// Check if this is an authentication message
    pub fn is_auth_message(&self) -> bool {
        matches!(self.message_type.as_str(), "auth" | "authenticate" | "authenticated" | "auth_failed")
    }
}

impl OutgoingMessage {
    /// Create a new outgoing message
    pub fn new(event_type: String, data: serde_json::Value) -> Self {
        Self {
            event_type,
            data,
            message_id: Some(Uuid::new_v4().to_string()),
            reply_to: None,
            timestamp: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            ),
            timeout_ms: None,
            requires_ack: None,
        }
    }
    
    /// Create a message with a specific ID
    pub fn with_id(mut self, id: String) -> Self {
        self.message_id = Some(id);
        self
    }
    
    /// Set reply_to field for acknowledgment
    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }
    
    /// Set timeout for this message
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
    
    /// Mark this message as requiring acknowledgment
    pub fn requires_acknowledgment(mut self) -> Self {
        self.requires_ack = Some(true);
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
    
    /// Create acknowledgment success response
    pub fn ack_success(reply_to: String, result_data: serde_json::Value) -> Self {
        Self::new("command_result".to_string(), serde_json::json!({
            "success": true,
            "result": result_data
        })).with_reply_to(reply_to)
    }
    
    /// Create acknowledgment error response
    pub fn ack_error(reply_to: String, error_message: String) -> Self {
        Self::new("command_result".to_string(), serde_json::json!({
            "success": false,
            "error": error_message
        })).with_reply_to(reply_to)
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
    
    /// Build custom event with acknowledgment required
    pub fn custom_event_with_ack(event_type: String, data: serde_json::Value, timeout_ms: Option<u64>) -> OutgoingMessage {
        let mut msg = OutgoingMessage::new(event_type, data).requires_acknowledgment();
        if let Some(timeout) = timeout_ms {
            msg = msg.with_timeout(timeout);
        }
        msg
    }
    
    /// Build acknowledgment response
    pub fn ack_response(reply_to: String, success: bool, data: serde_json::Value) -> OutgoingMessage {
        if success {
            OutgoingMessage::ack_success(reply_to, data)
        } else {
            let error_msg = data.get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            OutgoingMessage::ack_error(reply_to, error_msg)
        }
    }
}

#[cfg(feature = "native")]
/// Response handler for tracking request/response pairs with timeout and retry support
pub struct ResponseTracker {
    pending_requests: std::collections::HashMap<String, PendingRequest>,
}

#[cfg(feature = "native")]
struct PendingRequest {
    sender: tokio::sync::oneshot::Sender<IncomingMessage>,
    created_at: std::time::Instant,
    timeout_ms: u64,
    retry_count: u8,
    max_retries: u8,
    original_message: OutgoingMessage,
}

#[cfg(feature = "native")]
impl ResponseTracker {
    pub fn new() -> Self {
        Self {
            pending_requests: std::collections::HashMap::new(),
        }
    }
    
    /// Add a pending request with timeout and retry support
    pub fn add_request(&mut self, message: OutgoingMessage, sender: tokio::sync::oneshot::Sender<IncomingMessage>) {
        if let Some(message_id) = message.message_id.clone() {
            let timeout_ms = message.timeout_ms.unwrap_or(30000); // Default 30 seconds
            let pending_request = PendingRequest {
                sender,
                created_at: std::time::Instant::now(),
                timeout_ms,
                retry_count: 0,
                max_retries: 3,
                original_message: message,
            };
            self.pending_requests.insert(message_id, pending_request);
        }
    }
    
    /// Add a simple request (backwards compatibility)
    pub fn add_simple_request(&mut self, message_id: String, sender: tokio::sync::oneshot::Sender<IncomingMessage>) {
        let pending_request = PendingRequest {
            sender,
            created_at: std::time::Instant::now(),
            timeout_ms: 30000,
            retry_count: 0,
            max_retries: 0, // No retries for simple requests
            original_message: OutgoingMessage::new("unknown".to_string(), serde_json::Value::Null),
        };
        self.pending_requests.insert(message_id, pending_request);
    }
    
    /// Handle incoming response
    pub fn handle_response(&mut self, message: &IncomingMessage) -> bool {
        // Handle both direct message_id matches and reply_to matches
        let lookup_id = message.reply_to.as_ref()
            .or(message.message_id.as_ref());
            
        if let Some(id) = lookup_id {
            if let Some(pending_request) = self.pending_requests.remove(id) {
                let _ = pending_request.sender.send(message.clone());
                return true;
            }
        }
        false
    }
    
    /// Clean up expired requests and handle retries
    pub fn cleanup_expired(&mut self) -> Vec<OutgoingMessage> {
        let now = std::time::Instant::now();
        let mut expired_keys = Vec::new();
        let mut retry_messages = Vec::new();
        
        for (key, request) in &mut self.pending_requests {
            let elapsed_ms = now.duration_since(request.created_at).as_millis() as u64;
            
            if elapsed_ms >= request.timeout_ms {
                // Request has timed out
                if request.retry_count < request.max_retries {
                    // Retry the request
                    request.retry_count += 1;
                    request.created_at = now; // Reset timeout timer
                    
                    // Create retry message with same ID
                    let mut retry_message = request.original_message.clone();
                    if let Some(timeout) = retry_message.timeout_ms {
                        retry_message.timeout_ms = Some(timeout * 2); // Exponential backoff
                    }
                    retry_messages.push(retry_message);
                } else {
                    // Max retries exceeded, mark for removal
                    expired_keys.push(key.clone());
                }
            }
        }
        
        // Remove expired requests and send timeout responses
        for key in expired_keys {
            if let Some(request) = self.pending_requests.remove(&key) {
                let timeout_response = IncomingMessage {
                    message_type: "timeout".to_string(),
                    data: serde_json::json!({
                        "error": "Request timeout after retries",
                        "retry_count": request.retry_count
                    }),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                        .as_secs(),
                    success: Some(false),
                    error: Some("Request timeout after retries".to_string()),
                    message_id: Some(key),
                    reply_to: None,
                };
                
                let _ = request.sender.send(timeout_response);
            }
        }
        
        retry_messages
    }
    
    /// Get retry statistics
    pub fn get_retry_stats(&self) -> (usize, usize) {
        let pending_count = self.pending_requests.len();
        let retrying_count = self.pending_requests
            .values()
            .filter(|req| req.retry_count > 0)
            .count();
        (pending_count, retrying_count)
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
        assert_eq!(msg.reply_to, None);
        assert_eq!(msg.timeout_ms, None);
        assert_eq!(msg.requires_ack, None);
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
            reply_to: None,
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
            reply_to: None,
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
    
    #[test]
    fn test_acknowledgment_messages() {
        let ack_success = OutgoingMessage::ack_success(
            "test_id".to_string(),
            serde_json::json!({"result": "success"})
        );
        
        assert_eq!(ack_success.event_type, "command_result");
        assert_eq!(ack_success.reply_to, Some("test_id".to_string()));
        assert_eq!(ack_success.data["success"], true);
        
        let ack_error = OutgoingMessage::ack_error(
            "test_id".to_string(),
            "Test error".to_string()
        );
        
        assert_eq!(ack_error.event_type, "command_result");
        assert_eq!(ack_error.reply_to, Some("test_id".to_string()));
        assert_eq!(ack_error.data["success"], false);
        assert_eq!(ack_error.data["error"], "Test error");
    }
    
    #[test]
    fn test_message_with_acknowledgment() {
        let msg = OutgoingMessage::new(
            "test_command".to_string(),
            serde_json::json!({"param": "value"})
        )
        .requires_acknowledgment()
        .with_timeout(5000);
        
        assert_eq!(msg.requires_ack, Some(true));
        assert_eq!(msg.timeout_ms, Some(5000));
    }
}