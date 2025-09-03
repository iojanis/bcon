use serde::{Deserialize, Serialize};

/// Client role determines what operations are allowed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientRole {
    /// Guest users (unauthenticated, read-only access to public data)
    Guest,
    /// Regular players (authenticated, can send basic commands)
    Player,
    /// Administrators (can execute admin commands)
    Admin,
    /// System clients (full access, can communicate with adapters)
    System,
}

impl ClientRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientRole::Guest => "guest",
            ClientRole::Player => "player", 
            ClientRole::Admin => "admin",
            ClientRole::System => "system",
        }
    }
    
    /// Check if this role can send commands to adapters
    pub fn can_send_to_adapters(&self) -> bool {
        matches!(self, ClientRole::System)
    }
    
    /// Check if this role can receive all events
    pub fn can_receive_all_events(&self) -> bool {
        matches!(self, ClientRole::System | ClientRole::Admin)
    }
    
    /// Check if this role requires authentication
    pub fn requires_authentication(&self) -> bool {
        !matches!(self, ClientRole::Guest)
    }
}

/// Authentication configuration for different client roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthConfig {
    /// No authentication (guest access)
    Guest,
    
    /// JWT token authentication (all authenticated roles use tokens)
    Token {
        token: String,
        role: ClientRole,
    },
}

impl AuthConfig {
    /// Create system client authentication
    pub fn system(token: String) -> Self {
        Self::Token {
            token,
            role: ClientRole::System,
        }
    }
    
    /// Create admin authentication
    pub fn admin(token: String) -> Self {
        Self::Token {
            token,
            role: ClientRole::Admin,
        }
    }
    
    /// Create player authentication with token
    pub fn player(token: String) -> Self {
        Self::Token {
            token,
            role: ClientRole::Player,
        }
    }
    
    /// Get the expected role for this auth config
    pub fn expected_role(&self) -> ClientRole {
        match self {
            AuthConfig::Guest => ClientRole::Guest,
            AuthConfig::Token { role, .. } => role.clone(),
        }
    }
    
    /// Check if authentication is required
    pub fn requires_auth(&self) -> bool {
        !matches!(self, AuthConfig::Guest)
    }
}

/// Authentication message sent to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMessage {
    #[serde(rename = "eventType")]
    pub event_type: String,
    pub data: AuthData,
}

/// Authentication data payload - only JWT tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthData {
    pub token: String,
}

impl AuthMessage {
    /// Create authentication message from config
    pub fn from_config(config: &AuthConfig) -> Option<Self> {
        match config {
            AuthConfig::Guest => None,
            AuthConfig::Token { token, .. } => {
                Some(Self {
                    event_type: "auth".to_string(),
                    data: AuthData {
                        token: token.clone(),
                    },
                })
            },
        }
    }
}

/// Authentication response from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    #[serde(rename = "type")]
    pub message_type: String,
    pub success: bool,
    #[serde(rename = "socketId")]
    pub socket_id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: Option<String>,
    pub user: Option<UserInfo>,
    pub message: String,
}

/// User information received after successful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub role: String,
    #[serde(rename = "permissionLevel")]
    pub permission_level: Option<u32>,
}

impl AuthResponse {
    /// Check if authentication was successful
    pub fn is_success(&self) -> bool {
        self.success && self.message_type == "authenticated"
    }
    
    /// Get the authenticated role
    pub fn get_role(&self) -> Option<ClientRole> {
        self.user.as_ref().and_then(|user| {
            match user.role.as_str() {
                "guest" => Some(ClientRole::Guest),
                "player" => Some(ClientRole::Player),
                "admin" => Some(ClientRole::Admin),
                "system" => Some(ClientRole::System),
                _ => None,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        assert!(!ClientRole::Guest.can_send_to_adapters());
        assert!(!ClientRole::Player.can_send_to_adapters());
        assert!(!ClientRole::Admin.can_send_to_adapters());
        assert!(ClientRole::System.can_send_to_adapters());
        
        assert!(!ClientRole::Guest.can_receive_all_events());
        assert!(!ClientRole::Player.can_receive_all_events());
        assert!(ClientRole::Admin.can_receive_all_events());
        assert!(ClientRole::System.can_receive_all_events());
    }

    #[test]
    fn test_auth_config_creation() {
        let system_auth = AuthConfig::system("token123".to_string());
        assert_eq!(system_auth.expected_role(), ClientRole::System);
        
        let player_auth = AuthConfig::player("player_token".to_string());
        assert_eq!(player_auth.expected_role(), ClientRole::Player);
        
        let guest_auth = AuthConfig::Guest;
        assert_eq!(guest_auth.expected_role(), ClientRole::Guest);
        assert!(!guest_auth.requires_auth());
    }

    #[test]
    fn test_auth_message_creation() {
        let config = AuthConfig::Token {
            token: "test_token".to_string(),
            role: ClientRole::System,
        };
        
        let auth_msg = AuthMessage::from_config(&config).unwrap();
        assert_eq!(auth_msg.event_type, "auth");
        assert_eq!(auth_msg.data.token, "test_token");
    }
}