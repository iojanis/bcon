use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
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
    pub requires_ack: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "replyTo")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_ack: Option<bool>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    #[serde(rename = "type")]
    pub message_type: String,
    pub success: bool,
    #[serde(rename = "socketId")]
    pub socket_id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    #[serde(rename = "permissionLevel")]
    pub permission_level: Option<u32>,
}


impl IncomingMessage {
    pub fn new(event_type: String, data: serde_json::Value) -> Self {
        Self {
            event_type,
            data,
            message_id: None,
            reply_to: None,
            timestamp: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
            timeout_ms: None,
            requires_ack: None,
        }
    }

    pub fn with_message_id(mut self, message_id: String) -> Self {
        self.message_id = Some(message_id);
        self
    }

    pub fn is_auth_message(&self) -> bool {
        self.event_type == "auth"
    }

    pub fn extract_auth_data(&self) -> Result<AuthData, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

impl OutgoingMessage {
    pub fn new(message_type: String, data: serde_json::Value) -> Self {
        Self {
            message_type,
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: None,
            error: None,
            message_id: None,
            reply_to: None,
            timeout_ms: None,
            requires_ack: None,
        }
    }

    pub fn success(message_type: String, data: serde_json::Value) -> Self {
        Self {
            message_type,
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: Some(true),
            error: None,
            message_id: None,
            reply_to: None,
            timeout_ms: None,
            requires_ack: None,
        }
    }

    pub fn error(message_type: String, error: String) -> Self {
        Self {
            message_type,
            data: serde_json::Value::Null,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            success: Some(false),
            error: Some(error),
            message_id: None,
            reply_to: None,
            timeout_ms: None,
            requires_ack: None,
        }
    }

    pub fn with_message_id(mut self, message_id: String) -> Self {
        self.message_id = Some(message_id);
        self
    }

    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self.requires_ack = Some(true);
        self
    }

    pub fn requires_acknowledgment(mut self) -> Self {
        self.requires_ack = Some(true);
        self
    }

    /// Create an acknowledgment response for a message
    pub fn ack_success(reply_to: String, result_data: serde_json::Value) -> Self {
        Self::success("command_result".to_string(), result_data)
            .with_reply_to(reply_to)
    }

    /// Create an error acknowledgment response 
    pub fn ack_error(reply_to: String, error: String) -> Self {
        Self::error("command_result".to_string(), error)
            .with_reply_to(reply_to)
    }

    pub fn auth_success(socket_id: String, connection_id: String, user: UserInfo) -> Self {
        let username = user.username.clone();
        let response = AuthResponse {
            message_type: "authenticated".to_string(),
            success: true,
            socket_id,
            connection_id: Some(connection_id),
            user: Some(user),
            message: format!("Logged in as {}", username),
        };

        Self::success(
            "authenticated".to_string(),
            serde_json::to_value(response).unwrap_or(serde_json::Value::Null),
        )
    }

    pub fn auth_failed(socket_id: String, message: String) -> Self {
        let response = AuthResponse {
            message_type: "auth_failed".to_string(),
            success: false,
            socket_id,
            connection_id: None,
            user: None,
            message,
        };

        Self::error(
            "auth_failed".to_string(),
            serde_json::to_string(&response).unwrap_or_else(|_| "Authentication failed".to_string()),
        )
    }
}



impl RelayMessage {
    pub fn new(message_type: String, data: serde_json::Value, source_id: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_id,
        }
    }

    pub fn with_id(mut self, id: String) -> Self {
        self.id = id;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incoming_message_creation() {
        let msg = IncomingMessage::new(
            "test_event".to_string(),
            serde_json::json!({"key": "value"}),
        );
        
        assert_eq!(msg.event_type, "test_event");
        assert!(msg.timestamp.is_some());
    }

    #[test]
    fn test_outgoing_message_success() {
        let msg = OutgoingMessage::success(
            "test_response".to_string(),
            serde_json::json!({"result": "ok"}),
        );
        
        assert_eq!(msg.message_type, "test_response");
        assert_eq!(msg.success, Some(true));
        assert!(msg.error.is_none());
    }

    #[test]
    fn test_outgoing_message_error() {
        let msg = OutgoingMessage::error(
            "test_error".to_string(),
            "Something went wrong".to_string(),
        );
        
        assert_eq!(msg.message_type, "test_error");
        assert_eq!(msg.success, Some(false));
        assert_eq!(msg.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_auth_message_detection() {
        let auth_msg = IncomingMessage::new(
            "auth".to_string(),
            serde_json::json!({"token": "test_token"}),
        );
        
        assert!(auth_msg.is_auth_message());
    }

    #[test]
    fn test_relay_message_creation() {
        let msg = RelayMessage::new(
            "test_relay".to_string(),
            serde_json::json!({"data": "test"}),
            Some("source_123".to_string()),
        );
        
        assert_eq!(msg.message_type, "test_relay");
        assert_eq!(msg.source_id, Some("source_123".to_string()));
        assert!(!msg.id.is_empty());
    }
}