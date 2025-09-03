use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Missing server ID in adapter token")]
    MissingServerId,
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientRole {
    Guest,
    Player,
    Admin,
    System,
}

impl ClientRole {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "guest" => ClientRole::Guest,
            "player" => ClientRole::Player,
            "admin" => ClientRole::Admin,
            "operator" => ClientRole::Admin, // Legacy compatibility
            "system" => ClientRole::System,
            _ => ClientRole::Guest,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ClientRole::Guest => "guest",
            ClientRole::Player => "player",
            ClientRole::Admin => "admin",
            ClientRole::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterTokenClaims {
    pub server_id: String,
    pub server_name: Option<String>,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientTokenClaims {
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct ValidatedAdapterToken {
    pub server_id: String,
    pub server_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedClientToken {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub role: ClientRole,
}

pub struct AuthService {
    adapter_encoding_key: EncodingKey,
    adapter_decoding_key: DecodingKey,
    client_encoding_key: EncodingKey,
    client_decoding_key: DecodingKey,
    failure_count: AtomicU64,
}

impl AuthService {
    pub fn new(adapter_secret: String, client_secret: String) -> Result<Self> {
        Ok(Self {
            adapter_encoding_key: EncodingKey::from_secret(adapter_secret.as_bytes()),
            adapter_decoding_key: DecodingKey::from_secret(adapter_secret.as_bytes()),
            client_encoding_key: EncodingKey::from_secret(client_secret.as_bytes()),
            client_decoding_key: DecodingKey::from_secret(client_secret.as_bytes()),
            failure_count: AtomicU64::new(0),
        })
    }

    pub fn create_adapter_token(
        &self,
        server_id: String,
        server_name: Option<String>,
        expires_in_days: i64,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::days(expires_in_days);

        let claims = AdapterTokenClaims {
            server_id,
            server_name,
            iss: "bcon-server".to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.adapter_encoding_key)
            .map_err(AuthError::from)
            .map_err(Into::into)
    }

    pub fn verify_adapter_token(&self, token: &str) -> Result<ValidatedAdapterToken> {
        let validation = Validation::default();
        
        match decode::<AdapterTokenClaims>(token, &self.adapter_decoding_key, &validation) {
            Ok(token_data) => {
                let claims = token_data.claims;
                
                // Check if token is expired
                let now = Utc::now().timestamp();
                if claims.exp < now {
                    self.failure_count.fetch_add(1, Ordering::Relaxed);
                    return Err(AuthError::TokenExpired.into());
                }

                if claims.server_id.is_empty() {
                    self.failure_count.fetch_add(1, Ordering::Relaxed);
                    return Err(AuthError::MissingServerId.into());
                }

                Ok(ValidatedAdapterToken {
                    server_id: claims.server_id,
                    server_name: claims.server_name,
                })
            }
            Err(e) => {
                self.failure_count.fetch_add(1, Ordering::Relaxed);
                Err(AuthError::InvalidToken(e.to_string()).into())
            }
        }
    }

    pub fn create_client_token(
        &self,
        user_id: Option<String>,
        username: Option<String>,
        role: ClientRole,
        expires_in_hours: i64,
    ) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(expires_in_hours);

        let claims = ClientTokenClaims {
            user_id,
            name: username,
            role: role.as_str().to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.client_encoding_key)
            .map_err(AuthError::from)
            .map_err(Into::into)
    }

    pub fn verify_client_token(&self, token: &str) -> Result<ValidatedClientToken> {
        let validation = Validation::default();
        
        match decode::<ClientTokenClaims>(token, &self.client_decoding_key, &validation) {
            Ok(token_data) => {
                let claims = token_data.claims;
                
                // Check if token is expired
                let now = Utc::now().timestamp();
                if claims.exp < now {
                    self.failure_count.fetch_add(1, Ordering::Relaxed);
                    return Err(AuthError::TokenExpired.into());
                }

                Ok(ValidatedClientToken {
                    user_id: claims.user_id,
                    username: claims.name,
                    role: ClientRole::from_str(&claims.role),
                })
            }
            Err(e) => {
                self.failure_count.fetch_add(1, Ordering::Relaxed);
                Err(AuthError::InvalidToken(e.to_string()).into())
            }
        }
    }

    pub fn get_failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::Relaxed)
    }

    pub fn reset_failure_count(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_token_creation_and_verification() {
        let auth = AuthService::new(
            "adapter_secret".to_string(),
            "client_secret".to_string(),
        ).unwrap();

        let token = auth.create_adapter_token(
            "server1".to_string(),
            Some("Test Server".to_string()),
            30,
        ).unwrap();

        let validated = auth.verify_adapter_token(&token).unwrap();
        assert_eq!(validated.server_id, "server1");
        assert_eq!(validated.server_name, Some("Test Server".to_string()));
    }

    #[test]
    fn test_client_token_creation_and_verification() {
        let auth = AuthService::new(
            "adapter_secret".to_string(),
            "client_secret".to_string(),
        ).unwrap();

        let token = auth.create_client_token(
            Some("user123".to_string()),
            Some("TestUser".to_string()),
            ClientRole::Player,
            24,
        ).unwrap();

        let validated = auth.verify_client_token(&token).unwrap();
        assert_eq!(validated.user_id, Some("user123".to_string()));
        assert_eq!(validated.username, Some("TestUser".to_string()));
        assert_eq!(validated.role, ClientRole::Player);
    }

    #[test]
    fn test_invalid_token() {
        let auth = AuthService::new(
            "adapter_secret".to_string(),
            "client_secret".to_string(),
        ).unwrap();

        let result = auth.verify_adapter_token("invalid_token");
        assert!(result.is_err());
        assert!(auth.get_failure_count() > 0);
    }
}