use thiserror::Error;

#[derive(Error, Debug)]
pub enum BconError {
    #[error("Authentication error: {0}")]
    Authentication(#[from] crate::auth::AuthError),
    
    #[error("Rate limit error: {0}")]
    RateLimit(#[from] crate::rate_limiter::RateLimitError),
    
    #[error("Key-value store error: {0}")]
    KvStore(#[from] crate::kv_store::KvError),
    
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Message routing error: {0}")]
    MessageRouting(String),
    
    #[error("Server error: {0}")]
    Server(String),
    
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
    
    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl BconError {
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }
    
    pub fn connection<S: Into<String>>(msg: S) -> Self {
        Self::Connection(msg.into())
    }
    
    pub fn message_routing<S: Into<String>>(msg: S) -> Self {
        Self::MessageRouting(msg.into())
    }
    
    pub fn server<S: Into<String>>(msg: S) -> Self {
        Self::Server(msg.into())
    }
    
    pub fn invalid_message<S: Into<String>>(msg: S) -> Self {
        Self::InvalidMessage(msg.into())
    }
    
    pub fn connection_not_found<S: Into<String>>(id: S) -> Self {
        Self::ConnectionNotFound(id.into())
    }
    
    pub fn permission_denied<S: Into<String>>(msg: S) -> Self {
        Self::PermissionDenied(msg.into())
    }
    
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Self::NotFound(msg.into())
    }
    
    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }
    
    pub fn error_code(&self) -> &'static str {
        match self {
            BconError::Authentication(_) => "AUTH_ERROR",
            BconError::RateLimit(_) => "RATE_LIMIT_ERROR",
            BconError::KvStore(_) => "STORAGE_ERROR",
            BconError::WebSocket(_) => "WEBSOCKET_ERROR",
            BconError::Json(_) => "JSON_ERROR",
            BconError::Io(_) => "IO_ERROR",
            BconError::Config(_) => "CONFIG_ERROR",
            BconError::Connection(_) => "CONNECTION_ERROR",
            BconError::MessageRouting(_) => "ROUTING_ERROR",
            BconError::Server(_) => "SERVER_ERROR",
            BconError::InvalidMessage(_) => "INVALID_MESSAGE",
            BconError::ConnectionNotFound(_) => "CONNECTION_NOT_FOUND",
            BconError::PermissionDenied(_) => "PERMISSION_DENIED",
            BconError::NotFound(_) => "NOT_FOUND",
            BconError::Internal(_) => "INTERNAL_ERROR",
        }
    }
    
    pub fn is_client_error(&self) -> bool {
        matches!(self,
            BconError::Authentication(_) |
            BconError::InvalidMessage(_) |
            BconError::PermissionDenied(_) |
            BconError::NotFound(_) |
            BconError::RateLimit(_)
        )
    }
    
    pub fn is_server_error(&self) -> bool {
        !self.is_client_error()
    }
    
    pub fn to_response_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": true,
            "code": self.error_code(),
            "message": self.to_string(),
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })
    }
}

pub type BconResult<T> = Result<T, BconError>;

// Conversion from anyhow::Error for compatibility
impl From<anyhow::Error> for BconError {
    fn from(err: anyhow::Error) -> Self {
        BconError::Internal(err.to_string())
    }
}

// Helper trait for converting Results
pub trait BconResultExt<T> {
    fn with_context<C, F>(self, f: F) -> BconResult<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T, E> BconResultExt<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context<C, F>(self, f: F) -> BconResult<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|e| BconError::Internal(format!("{}: {}", f().into(), e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let auth_error = BconError::Authentication(crate::auth::AuthError::InvalidToken("test".to_string()));
        assert_eq!(auth_error.error_code(), "AUTH_ERROR");
        assert!(auth_error.is_client_error());
        
        let config_error = BconError::config("Invalid configuration");
        assert_eq!(config_error.error_code(), "CONFIG_ERROR");
        assert!(config_error.is_server_error());
    }
    
    #[test]
    fn test_error_response_json() {
        let error = BconError::invalid_message("Bad JSON format");
        let json = error.to_response_json();
        
        assert_eq!(json["error"], true);
        assert_eq!(json["code"], "INVALID_MESSAGE");
        assert_eq!(json["message"], "Invalid message format: Bad JSON format");
        assert!(json["timestamp"].is_u64());
    }
    
    #[test]
    fn test_error_classification() {
        assert!(BconError::Authentication(crate::auth::AuthError::InvalidToken("".to_string())).is_client_error());
        assert!(BconError::PermissionDenied("Access denied".to_string()).is_client_error());
        assert!(BconError::InvalidMessage("Bad format".to_string()).is_client_error());
        
        assert!(BconError::Internal("Server panic".to_string()).is_server_error());
        assert!(BconError::Config("Invalid config".to_string()).is_server_error());
        assert!(BconError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")).is_server_error());
    }
    
    #[test]
    fn test_result_ext() {
        let result: Result<i32, std::io::Error> = Err(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let bcon_result: BconResult<i32> = result.with_context(|| "Failed to process data");
        
        assert!(bcon_result.is_err());
        match bcon_result {
            Err(BconError::Internal(msg)) => {
                assert!(msg.contains("Failed to process data"));
                assert!(msg.contains("test error"));
            }
            _ => panic!("Expected Internal error"),
        }
    }
}