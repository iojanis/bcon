use crate::auth::ClientRole;
use crate::kv_store::KvStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for {0}")]
    RateLimitExceeded(String),
    #[error("IP address banned: {0}")]
    IpBanned(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub guest_requests_per_minute: u32,
    pub player_requests_per_minute: u32,
    pub admin_requests_per_minute: u32,
    pub system_requests_per_minute: u32,
    pub unauthenticated_adapter_attempts_per_minute: u32,
    pub window_duration_seconds: u64,
    pub ban_threshold: u32,
    pub ban_duration_hours: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            guest_requests_per_minute: 30,
            player_requests_per_minute: 120,
            admin_requests_per_minute: 300,
            system_requests_per_minute: 1000,
            unauthenticated_adapter_attempts_per_minute: 5,
            window_duration_seconds: 60,
            ban_threshold: 50,
            ban_duration_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RateLimitEntry {
    count: u32,
    window_start: u64,
    first_request_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BanEntry {
    banned_at: u64,
    reason: String,
    expires_at: u64,
}

pub struct RateLimiter {
    config: RateLimitConfig,
    kv_store: Arc<KvStore>,
    error_count: AtomicU64,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig, kv_store: Arc<KvStore>) -> Self {
        Self {
            config,
            kv_store,
            error_count: AtomicU64::new(0),
        }
    }

    fn get_rate_limit_key(&self, ip: &str, context: &str) -> String {
        format!("rate_limit:{}:{}", ip, context)
    }

    fn get_ban_key(&self, ip: &str) -> String {
        format!("ban:{}", ip)
    }

    fn get_current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn get_limit_for_role(&self, role: &ClientRole) -> u32 {
        match role {
            ClientRole::Guest => self.config.guest_requests_per_minute,
            ClientRole::Player => self.config.player_requests_per_minute,
            ClientRole::Admin => self.config.admin_requests_per_minute,
            ClientRole::System => self.config.system_requests_per_minute,
        }
    }

    pub async fn is_ip_banned(&self, ip: &str) -> Result<bool> {
        let ban_key = self.get_ban_key(ip);
        
        if let Some(ban_entry) = self.kv_store.get_json::<BanEntry>(&ban_key)? {
            let now = self.get_current_timestamp();
            if ban_entry.expires_at > now {
                return Ok(true);
            } else {
                // Ban expired, remove it
                self.kv_store.delete(&ban_key)?;
            }
        }
        
        Ok(false)
    }

    pub async fn ban_ip(&self, ip: &str, reason: String) -> Result<()> {
        let ban_key = self.get_ban_key(ip);
        let now = self.get_current_timestamp();
        
        let ban_entry = BanEntry {
            banned_at: now,
            reason,
            expires_at: now + (self.config.ban_duration_hours as u64 * 3600),
        };

        self.kv_store.set_json(&ban_key, &ban_entry)?;
        warn!("Banned IP {} for {}", ip, ban_entry.reason);
        
        Ok(())
    }

    pub async fn unban_ip(&self, ip: &str) -> Result<()> {
        let ban_key = self.get_ban_key(ip);
        self.kv_store.delete(&ban_key)?;
        debug!("Unbanned IP {}", ip);
        Ok(())
    }

    pub async fn check_rate_limit(
        &self,
        ip: &str,
        role: &ClientRole,
        context: &str,
        cost: u32,
    ) -> Result<RateLimitResult> {
        // Check if IP is banned first
        if self.is_ip_banned(ip).await? {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return Ok(RateLimitResult::Banned);
        }

        let limit = self.get_limit_for_role(role);
        let key = self.get_rate_limit_key(ip, context);
        let now = self.get_current_timestamp();
        let window_start = now - (now % self.config.window_duration_seconds);

        let result = self.kv_store.atomic_update(&key, |current| {
            let mut entry = if let Some(current_value) = current {
                serde_json::from_value::<RateLimitEntry>(current_value.clone())
                    .unwrap_or_else(|_| RateLimitEntry {
                        count: 0,
                        window_start,
                        first_request_time: now,
                    })
            } else {
                RateLimitEntry {
                    count: 0,
                    window_start,
                    first_request_time: now,
                }
            };

            // Reset counter if we're in a new window
            if entry.window_start < window_start {
                entry.count = 0;
                entry.window_start = window_start;
            }

            // Check if adding this request would exceed the limit
            let new_count = entry.count + cost;
            if new_count > limit {
                // Check if we should ban this IP
                if entry.count > self.config.ban_threshold {
                    return Ok((serde_json::to_value(&entry)?, RateLimitResult::ShouldBan));
                }
                return Ok((serde_json::to_value(&entry)?, RateLimitResult::Exceeded { 
                    limit,
                    current: entry.count,
                    reset_time: window_start + self.config.window_duration_seconds,
                }));
            }

            // Update the count
            entry.count = new_count;
            Ok((serde_json::to_value(&entry)?, RateLimitResult::Allowed { 
                limit,
                remaining: limit - new_count,
                reset_time: window_start + self.config.window_duration_seconds,
            }))
        })?;

        // Handle banning if necessary
        if matches!(result, RateLimitResult::ShouldBan) {
            self.ban_ip(ip, format!("Excessive requests in {} context", context)).await?;
            return Ok(RateLimitResult::Banned);
        }

        // Track errors for exceeded limits
        if matches!(result, RateLimitResult::Exceeded { .. }) {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(result)
    }

    pub async fn check_client_rate_limit(
        &self,
        ip: &str,
        role: &ClientRole,
        message_type: &str,
    ) -> Result<RateLimitResult> {
        let cost = self.get_message_cost(message_type);
        let context = format!("client_{}", message_type);
        self.check_rate_limit(ip, role, &context, cost).await
    }

    pub async fn check_adapter_rate_limit(
        &self,
        ip: &str,
        is_authenticated: bool,
    ) -> Result<RateLimitResult> {
        if is_authenticated {
            // Authenticated adapters get system-level limits
            self.check_rate_limit(ip, &ClientRole::System, "adapter_connection", 1).await
        } else {
            // Unauthenticated adapters get very strict limits
            let limit = self.config.unauthenticated_adapter_attempts_per_minute;
            let now = self.get_current_timestamp();
            let window_start = now - (now % self.config.window_duration_seconds);
            let key = self.get_rate_limit_key(ip, "unauthenticated_adapter");

            let result = self.kv_store.atomic_update(&key, |current| {
                let mut entry = if let Some(current_value) = current {
                    serde_json::from_value::<RateLimitEntry>(current_value.clone())
                        .unwrap_or_else(|_| RateLimitEntry {
                            count: 0,
                            window_start,
                            first_request_time: now,
                        })
                } else {
                    RateLimitEntry {
                        count: 0,
                        window_start,
                        first_request_time: now,
                    }
                };

                // Reset counter if we're in a new window
                if entry.window_start < window_start {
                    entry.count = 0;
                    entry.window_start = window_start;
                }

                let new_count = entry.count + 1;
                if new_count > limit {
                    // Auto-ban for excessive unauthenticated adapter attempts
                    return Ok((serde_json::to_value(&entry)?, RateLimitResult::ShouldBan));
                }

                entry.count = new_count;
                Ok((serde_json::to_value(&entry)?, RateLimitResult::Allowed {
                    limit,
                    remaining: limit - new_count,
                    reset_time: window_start + self.config.window_duration_seconds,
                }))
            })?;

            if matches!(result, RateLimitResult::ShouldBan) {
                self.ban_ip(ip, "Excessive unauthenticated adapter connection attempts".to_string()).await?;
                return Ok(RateLimitResult::Banned);
            }

            if matches!(result, RateLimitResult::Exceeded { .. }) {
                self.error_count.fetch_add(1, Ordering::Relaxed);
            }

            Ok(result)
        }
    }

    fn get_message_cost(&self, message_type: &str) -> u32 {
        match message_type {
            "auth" => 3,
            "heartbeat" | "ping" => 1,
            "chat_message" => 2,
            "command" => 5,
            "admin_command" => 8,
            _ => 1,
        }
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    pub fn reset_error_count(&self) {
        self.error_count.store(0, Ordering::Relaxed);
    }

    pub async fn get_rate_limit_info(&self, ip: &str, context: &str) -> Result<Option<RateLimitInfo>> {
        let key = self.get_rate_limit_key(ip, context);
        
        if let Some(entry) = self.kv_store.get_json::<RateLimitEntry>(&key)? {
            let now = self.get_current_timestamp();
            let window_start = now - (now % self.config.window_duration_seconds);
            
            // Check if entry is from current window
            if entry.window_start >= window_start {
                return Ok(Some(RateLimitInfo {
                    current_count: entry.count,
                    window_start: entry.window_start,
                    reset_time: window_start + self.config.window_duration_seconds,
                }));
            }
        }
        
        Ok(None)
    }

    pub async fn cleanup_expired_entries(&self) -> Result<u32> {
        let now = self.get_current_timestamp();
        let mut cleaned = 0;

        // Clean up expired ban entries
        let ban_keys = self.kv_store.keys_with_prefix("ban:");
        for key in ban_keys {
            if let Some(ban_entry) = self.kv_store.get_json::<BanEntry>(&key)? {
                if ban_entry.expires_at <= now {
                    self.kv_store.delete(&key)?;
                    cleaned += 1;
                }
            }
        }

        // Clean up old rate limit entries (older than 2 windows)
        let rate_limit_keys = self.kv_store.keys_with_prefix("rate_limit:");
        let cutoff_time = now - (self.config.window_duration_seconds * 2);
        
        for key in rate_limit_keys {
            if let Some(entry) = self.kv_store.get_json::<RateLimitEntry>(&key)? {
                if entry.window_start < cutoff_time {
                    self.kv_store.delete(&key)?;
                    cleaned += 1;
                }
            }
        }

        if cleaned > 0 {
            debug!("Cleaned up {} expired rate limit entries", cleaned);
        }

        Ok(cleaned)
    }
}

#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed {
        limit: u32,
        remaining: u32,
        reset_time: u64,
    },
    Exceeded {
        limit: u32,
        current: u32,
        reset_time: u64,
    },
    Banned,
    ShouldBan,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    pub current_count: u32,
    pub window_start: u64,
    pub reset_time: u64,
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }

    pub fn to_error_message(&self) -> Option<String> {
        match self {
            RateLimitResult::Exceeded { limit, current, reset_time } => {
                Some(format!(
                    "Rate limit exceeded: {}/{} requests. Reset at {}",
                    current, limit, reset_time
                ))
            }
            RateLimitResult::Banned => {
                Some("IP address is banned".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let kv_store = Arc::new(KvStore::new());
        let config = RateLimitConfig {
            guest_requests_per_minute: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config, kv_store);

        // Should allow initial requests
        let result = limiter.check_client_rate_limit("127.0.0.1", &ClientRole::Guest, "test").await.unwrap();
        assert!(result.is_allowed());

        // Should exceed limit after multiple requests
        for _ in 0..5 {
            limiter.check_client_rate_limit("127.0.0.1", &ClientRole::Guest, "test").await.unwrap();
        }

        let result = limiter.check_client_rate_limit("127.0.0.1", &ClientRole::Guest, "test").await.unwrap();
        assert!(!result.is_allowed());
    }

    #[tokio::test]
    async fn test_ip_banning() {
        let kv_store = Arc::new(KvStore::new());
        let limiter = RateLimiter::new(RateLimitConfig::default(), kv_store);

        assert!(!limiter.is_ip_banned("192.168.1.1").await.unwrap());

        limiter.ban_ip("192.168.1.1", "Test ban".to_string()).await.unwrap();
        assert!(limiter.is_ip_banned("192.168.1.1").await.unwrap());

        limiter.unban_ip("192.168.1.1").await.unwrap();
        assert!(!limiter.is_ip_banned("192.168.1.1").await.unwrap());
    }

    #[tokio::test]
    async fn test_unauthenticated_adapter_limiting() {
        let kv_store = Arc::new(KvStore::new());
        let config = RateLimitConfig {
            unauthenticated_adapter_attempts_per_minute: 2,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config, kv_store);

        // Should allow initial attempts
        let result = limiter.check_adapter_rate_limit("10.0.0.1", false).await.unwrap();
        assert!(result.is_allowed());

        // Should ban after exceeding limit
        limiter.check_adapter_rate_limit("10.0.0.1", false).await.unwrap();
        let result = limiter.check_adapter_rate_limit("10.0.0.1", false).await.unwrap();
        assert!(matches!(result, RateLimitResult::Banned | RateLimitResult::ShouldBan));
    }
}