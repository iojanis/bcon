use crate::message::{IncomingMessage, OutgoingMessage};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub id: String,
    pub reply_to: Option<String>,
    pub created_at: Instant,
    pub timeout_ms: u64,
    pub retry_count: u8,
    pub max_retries: u8,
    pub connection_id: String,
    pub command_type: String,
}

#[derive(Debug)]
pub struct CommandResult {
    pub command_id: String,
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

pub struct CommandTracker {
    pending_commands: Arc<DashMap<String, PendingCommand>>,
    timeout_checker: Option<tokio::task::JoinHandle<()>>,
    command_counter: AtomicU64,
}

impl CommandTracker {
    pub fn new() -> Self {
        Self {
            pending_commands: Arc::new(DashMap::new()),
            timeout_checker: None,
            command_counter: AtomicU64::new(0),
        }
    }

    pub fn start_timeout_checker(&mut self) {
        let pending_commands = Arc::clone(&self.pending_commands);
        
        self.timeout_checker = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(1000));
            
            loop {
                interval.tick().await;
                Self::check_timeouts(&pending_commands).await;
            }
        }));
    }

    /// Generate a unique command ID
    pub fn generate_command_id(&self) -> String {
        let count = self.command_counter.fetch_add(1, Ordering::Relaxed);
        format!("cmd_{}_{}", 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            count
        )
    }

    /// Track a command that requires acknowledgment
    pub fn track_command(&self, message: &OutgoingMessage, connection_id: String) -> Option<String> {
        if message.requires_ack != Some(true) {
            return None;
        }

        let command_id = message.message_id.clone().unwrap_or_else(|| self.generate_command_id());
        let timeout_ms = message.timeout_ms.unwrap_or(30000); // 30 second default

        let pending_command = PendingCommand {
            id: command_id.clone(),
            reply_to: message.reply_to.clone(),
            created_at: Instant::now(),
            timeout_ms,
            retry_count: 0,
            max_retries: 3,
            connection_id,
            command_type: message.message_type.clone(),
        };

        self.pending_commands.insert(command_id.clone(), pending_command);
        debug!("Tracking command: {} (timeout: {}ms)", command_id, timeout_ms);

        Some(command_id)
    }

    /// Handle acknowledgment for a command
    pub fn handle_acknowledgment(&self, message: &IncomingMessage) -> Option<PendingCommand> {
        if let Some(reply_to) = &message.reply_to {
            if let Some((_key, pending_command)) = self.pending_commands.remove(reply_to) {
                info!("Command acknowledged: {} (took: {}ms)", 
                    reply_to,
                    pending_command.created_at.elapsed().as_millis()
                );
                return Some(pending_command);
            } else {
                warn!("Received acknowledgment for unknown command: {}", reply_to);
            }
        }
        None
    }

    /// Get all pending commands for a connection
    pub fn get_pending_for_connection(&self, connection_id: &str) -> Vec<PendingCommand> {
        self.pending_commands
            .iter()
            .filter(|entry| entry.value().connection_id == connection_id)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Remove all pending commands for a connection (on disconnect)
    pub fn cleanup_connection(&self, connection_id: &str) {
        let keys_to_remove: Vec<String> = self.pending_commands
            .iter()
            .filter(|entry| entry.value().connection_id == connection_id)
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            self.pending_commands.remove(&key);
        }

        debug!("Cleaned up pending commands for connection: {}", connection_id);
    }

    /// Get statistics
    pub fn get_stats(&self) -> CommandTrackerStats {
        CommandTrackerStats {
            pending_count: self.pending_commands.len() as u64,
            total_commands: self.command_counter.load(Ordering::Relaxed),
        }
    }

    async fn check_timeouts(pending_commands: &DashMap<String, PendingCommand>) {
        let now = Instant::now();
        let mut timed_out_commands = Vec::new();

        // Find timed out commands
        for entry in pending_commands.iter() {
            let pending_command = entry.value();
            let elapsed = now.duration_since(pending_command.created_at);
            
            if elapsed.as_millis() > pending_command.timeout_ms as u128 {
                timed_out_commands.push(pending_command.id.clone());
            }
        }

        // Remove timed out commands and log
        for command_id in timed_out_commands {
            if let Some((_key, timed_out_command)) = pending_commands.remove(&command_id) {
                warn!(
                    "Command timed out: {} (type: {}, elapsed: {}ms, timeout: {}ms)",
                    timed_out_command.id,
                    timed_out_command.command_type,
                    timed_out_command.created_at.elapsed().as_millis(),
                    timed_out_command.timeout_ms
                );

                // Here you could implement retry logic or send timeout notifications
            }
        }
    }
}

impl Drop for CommandTracker {
    fn drop(&mut self) {
        if let Some(handle) = self.timeout_checker.take() {
            handle.abort();
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandTrackerStats {
    pub pending_count: u64,
    pub total_commands: u64,
}

impl Default for CommandTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::OutgoingMessage;
    
    #[tokio::test]
    async fn test_command_tracking() {
        let tracker = CommandTracker::new();
        
        let message = OutgoingMessage::new("test_command".to_string(), serde_json::json!({}))
            .with_timeout(5000)
            .with_message_id("test_123".to_string());
            
        let command_id = tracker.track_command(&message, "conn_1".to_string());
        assert_eq!(command_id, Some("test_123".to_string()));
        
        let stats = tracker.get_stats();
        assert_eq!(stats.pending_count, 1);
    }
    
    #[tokio::test]
    async fn test_acknowledgment_handling() {
        let tracker = CommandTracker::new();
        
        // Track a command
        let message = OutgoingMessage::new("test_command".to_string(), serde_json::json!({}))
            .with_timeout(5000)
            .with_message_id("test_123".to_string());
            
        tracker.track_command(&message, "conn_1".to_string());
        
        // Acknowledge it
        let ack_message = IncomingMessage::new("command_result".to_string(), serde_json::json!({}))
            .with_message_id("ack_456".to_string());
        let mut ack = ack_message;
        ack.reply_to = Some("test_123".to_string());
        
        let acknowledged = tracker.handle_acknowledgment(&ack);
        assert!(acknowledged.is_some());
        
        let stats = tracker.get_stats();
        assert_eq!(stats.pending_count, 0);
    }
}