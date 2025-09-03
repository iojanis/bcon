use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, warn};

#[derive(Error, Debug)]
pub enum KvError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Value expired")]
    ValueExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvEntry {
    pub value: serde_json::Value,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub access_count: u64,
    pub last_accessed: u64,
}

impl KvEntry {
    pub fn new(value: serde_json::Value, ttl_seconds: Option<u64>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            value,
            created_at: now,
            expires_at: ttl_seconds.map(|ttl| now + ttl),
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > expires_at
        } else {
            false
        }
    }

    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

pub struct KvStore {
    data: DashMap<String, KvEntry>,
    cleanup_interval: Duration,
}

impl KvStore {
    pub fn new() -> Self {
        Self::new_with_cleanup_interval(Duration::from_secs(300)) // 5 minutes
    }

    pub fn new_with_cleanup_interval(cleanup_interval: Duration) -> Self {
        let store = Self {
            data: DashMap::new(),
            cleanup_interval,
        };

        // Start background cleanup task
        let data_clone = store.data.clone();
        let interval_duration = cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = interval(interval_duration);
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
            
            loop {
                interval_timer.tick().await;
                Self::cleanup_expired_entries(&data_clone).await;
            }
        });

        store
    }

    pub fn set(&self, key: &str, value: serde_json::Value) -> Result<()> {
        self.set_with_ttl(key, value, None)
    }

    pub fn set_with_ttl(&self, key: &str, value: serde_json::Value, ttl_seconds: Option<u64>) -> Result<()> {
        let entry = KvEntry::new(value, ttl_seconds);
        self.data.insert(key.to_string(), entry);
        debug!("KV: Set key '{}' with TTL {:?}", key, ttl_seconds);
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        if let Some(mut entry) = self.data.get_mut(key) {
            if entry.is_expired() {
                drop(entry); // Drop the mutable reference
                self.data.remove(key);
                return Err(KvError::ValueExpired.into());
            }
            entry.touch();
            Ok(Some(entry.value.clone()))
        } else {
            Ok(None)
        }
    }

    pub fn get_entry(&self, key: &str) -> Result<Option<KvEntry>> {
        if let Some(mut entry) = self.data.get_mut(key) {
            if entry.is_expired() {
                drop(entry);
                self.data.remove(key);
                return Err(KvError::ValueExpired.into());
            }
            entry.touch();
            Ok(Some(entry.clone()))
        } else {
            Ok(None)
        }
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.data.remove(key);
        debug!("KV: Deleted key '{}'", key);
        Ok(())
    }

    pub fn exists(&self, key: &str) -> bool {
        if let Some(entry) = self.data.get(key) {
            if entry.is_expired() {
                drop(entry);
                self.data.remove(key);
                false
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn keys(&self) -> Vec<String> {
        self.data.iter()
            .filter(|entry| !entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        self.data.iter()
            .filter(|entry| entry.key().starts_with(prefix) && !entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn clear(&self) {
        self.data.clear();
        debug!("KV: Cleared all entries");
    }

    pub fn size(&self) -> usize {
        // Filter out expired entries for accurate count
        self.data.iter()
            .filter(|entry| !entry.value().is_expired())
            .count()
    }

    pub fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let mut new_value = delta;
        
        if let Some(mut entry) = self.data.get_mut(key) {
            if entry.is_expired() {
                drop(entry);
                self.data.remove(key);
            } else {
                if let Some(current) = entry.value.as_i64() {
                    new_value = current + delta;
                    entry.value = serde_json::Value::Number(serde_json::Number::from(new_value));
                    entry.touch();
                    return Ok(new_value);
                } else {
                    return Err(anyhow::anyhow!("Value is not a number"));
                }
            }
        }

        // Create new entry if key doesn't exist or was expired
        self.set(key, serde_json::Value::Number(serde_json::Number::from(new_value)))?;
        Ok(new_value)
    }

    pub fn atomic_update<F, T>(&self, key: &str, updater: F) -> Result<T>
    where
        F: FnOnce(Option<&serde_json::Value>) -> Result<(serde_json::Value, T)>,
    {
        // This is a simplified atomic update - for true atomicity across multiple operations,
        // you'd need additional synchronization
        let current_value = if let Some(entry_ref) = self.data.get(key) {
            if entry_ref.is_expired() {
                drop(entry_ref);
                self.data.remove(key);
                None
            } else {
                Some(entry_ref.value.clone())
            }
        } else {
            None
        };

        let current_value_ref = current_value.as_ref();

        let (new_value, result) = updater(current_value_ref)?;
        self.set(key, new_value)?;
        Ok(result)
    }

    async fn cleanup_expired_entries(data: &DashMap<String, KvEntry>) {
        let mut expired_keys = Vec::new();
        
        // Collect expired keys
        for entry in data.iter() {
            if entry.value().is_expired() {
                expired_keys.push(entry.key().clone());
            }
        }

        let expired_count = expired_keys.len();
        
        // Remove expired entries
        for key in expired_keys {
            data.remove(&key);
        }

        if expired_count > 0 {
            debug!("KV: Cleaned up {} expired entries", expired_count);
        }
    }

    pub fn get_stats(&self) -> KvStats {
        let mut total_entries = 0;
        let mut expired_entries = 0;
        let mut total_accesses = 0;

        for entry in self.data.iter() {
            total_entries += 1;
            if entry.value().is_expired() {
                expired_entries += 1;
            } else {
                total_accesses += entry.value().access_count;
            }
        }

        KvStats {
            total_entries,
            expired_entries,
            active_entries: total_entries - expired_entries,
            total_accesses,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KvStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub active_entries: usize,
    pub total_accesses: u64,
}

impl Default for KvStore {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience methods for common data types
impl KvStore {
    pub fn set_string(&self, key: &str, value: String) -> Result<()> {
        self.set(key, serde_json::Value::String(value))
    }

    pub fn get_string(&self, key: &str) -> Result<Option<String>> {
        match self.get(key)? {
            Some(serde_json::Value::String(s)) => Ok(Some(s)),
            Some(_) => Err(anyhow::anyhow!("Value is not a string")),
            None => Ok(None),
        }
    }

    pub fn set_number(&self, key: &str, value: i64) -> Result<()> {
        self.set(key, serde_json::Value::Number(serde_json::Number::from(value)))
    }

    pub fn get_number(&self, key: &str) -> Result<Option<i64>> {
        match self.get(key)? {
            Some(serde_json::Value::Number(n)) => Ok(n.as_i64()),
            Some(_) => Err(anyhow::anyhow!("Value is not a number")),
            None => Ok(None),
        }
    }

    pub fn set_bool(&self, key: &str, value: bool) -> Result<()> {
        self.set(key, serde_json::Value::Bool(value))
    }

    pub fn get_bool(&self, key: &str) -> Result<Option<bool>> {
        match self.get(key)? {
            Some(serde_json::Value::Bool(b)) => Ok(Some(b)),
            Some(_) => Err(anyhow::anyhow!("Value is not a boolean")),
            None => Ok(None),
        }
    }

    pub fn set_json<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        self.set(key, json_value)
    }

    pub fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.get(key)? {
            Some(value) => Ok(Some(serde_json::from_value(value)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_basic_operations() {
        let kv = KvStore::new();
        
        // Test set/get
        kv.set_string("test_key", "test_value".to_string()).unwrap();
        assert_eq!(kv.get_string("test_key").unwrap(), Some("test_value".to_string()));
        
        // Test exists
        assert!(kv.exists("test_key"));
        assert!(!kv.exists("nonexistent_key"));
        
        // Test delete
        kv.delete("test_key").unwrap();
        assert!(!kv.exists("test_key"));
    }

    #[tokio::test]
    async fn test_ttl() {
        let kv = KvStore::new_with_cleanup_interval(Duration::from_millis(100));
        
        // Set with 1 second TTL
        kv.set_with_ttl("ttl_key", serde_json::Value::String("ttl_value".to_string()), Some(1)).unwrap();
        assert!(kv.exists("ttl_key"));
        
        // Wait for expiration
        sleep(Duration::from_secs(2)).await;
        assert!(!kv.exists("ttl_key"));
    }

    #[tokio::test]
    async fn test_increment() {
        let kv = KvStore::new();
        
        assert_eq!(kv.increment("counter", 5).unwrap(), 5);
        assert_eq!(kv.increment("counter", 3).unwrap(), 8);
        assert_eq!(kv.increment("counter", -2).unwrap(), 6);
    }

    #[test]
    fn test_json_operations() {
        let kv = KvStore::new();
        
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestStruct {
            name: String,
            age: u32,
        }
        
        let data = TestStruct {
            name: "Alice".to_string(),
            age: 25,
        };
        
        kv.set_json("user", &data).unwrap();
        let retrieved: TestStruct = kv.get_json("user").unwrap().unwrap();
        assert_eq!(retrieved, data);
    }
}