use crate::rate_limiter::RateLimitConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BconConfig {
    pub adapter_port: u16,
    pub client_port: u16,
    pub adapter_secret: String,
    pub client_secret: String,
    pub rate_limits: RateLimitConfig,
    pub allowed_origins: Vec<String>,
    pub heartbeat_interval_seconds: u64,
    pub connection_timeout_seconds: u64,
    pub log_level: String,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub url: String,
    pub minecraft_version: String,
}

impl Default for BconConfig {
    fn default() -> Self {
        Self {
            adapter_port: 8082,
            client_port: 8081,
            adapter_secret: Self::generate_secret(),
            client_secret: Self::generate_secret(),
            rate_limits: RateLimitConfig::default(),
            allowed_origins: vec!["*".to_string()],
            heartbeat_interval_seconds: 30,
            connection_timeout_seconds: 300,
            log_level: "info".to_string(),
            server_info: ServerInfo::default(),
        }
    }
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            name: "Bcon Server".to_string(),
            description: "WebSocket communication server for Minecraft adapters and clients".to_string(),
            url: "localhost".to_string(),
            minecraft_version: "1.20+".to_string(),
        }
    }
}

impl BconConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(port) = env::var("BCON_ADAPTER_PORT") {
            config.adapter_port = port.parse()?;
        }

        if let Ok(port) = env::var("BCON_CLIENT_PORT") {
            config.client_port = port.parse()?;
        }

        if let Ok(secret) = env::var("BCON_ADAPTER_SECRET") {
            config.adapter_secret = secret;
        }

        if let Ok(secret) = env::var("BCON_CLIENT_SECRET") {
            config.client_secret = secret;
        }

        if let Ok(level) = env::var("BCON_LOG_LEVEL") {
            config.log_level = level;
        }

        if let Ok(origins) = env::var("BCON_ALLOWED_ORIGINS") {
            config.allowed_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
        }

        if let Ok(interval) = env::var("BCON_HEARTBEAT_INTERVAL") {
            config.heartbeat_interval_seconds = interval.parse()?;
        }

        if let Ok(timeout) = env::var("BCON_CONNECTION_TIMEOUT") {
            config.connection_timeout_seconds = timeout.parse()?;
        }

        // Server info overrides
        if let Ok(name) = env::var("BCON_SERVER_NAME") {
            config.server_info.name = name;
        }

        if let Ok(desc) = env::var("BCON_SERVER_DESCRIPTION") {
            config.server_info.description = desc;
        }

        if let Ok(url) = env::var("BCON_SERVER_URL") {
            config.server_info.url = url;
        }

        if let Ok(version) = env::var("BCON_MINECRAFT_VERSION") {
            config.server_info.minecraft_version = version;
        }

        // Rate limit overrides
        if let Ok(limit) = env::var("BCON_GUEST_RATE_LIMIT") {
            config.rate_limits.guest_requests_per_minute = limit.parse()?;
        }

        if let Ok(limit) = env::var("BCON_PLAYER_RATE_LIMIT") {
            config.rate_limits.player_requests_per_minute = limit.parse()?;
        }

        if let Ok(limit) = env::var("BCON_ADMIN_RATE_LIMIT") {
            config.rate_limits.admin_requests_per_minute = limit.parse()?;
        }

        if let Ok(limit) = env::var("BCON_SYSTEM_RATE_LIMIT") {
            config.rate_limits.system_requests_per_minute = limit.parse()?;
        }

        if let Ok(limit) = env::var("BCON_UNAUTHENTICATED_ADAPTER_RATE_LIMIT") {
            config.rate_limits.unauthenticated_adapter_attempts_per_minute = limit.parse()?;
        }

        if let Ok(threshold) = env::var("BCON_BAN_THRESHOLD") {
            config.rate_limits.ban_threshold = threshold.parse()?;
        }

        if let Ok(duration) = env::var("BCON_BAN_DURATION_HOURS") {
            config.rate_limits.ban_duration_hours = duration.parse()?;
        }

        Ok(config)
    }

    pub fn from_file(path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(path)?;
        let config: BconConfig = if path.ends_with(".json") {
            serde_json::from_str(&config_str)?
        } else if path.ends_with(".toml") {
            toml::from_str(&config_str)?
        } else {
            return Err(anyhow::anyhow!("Unsupported config file format. Use .json or .toml"));
        };
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let config_str = if path.ends_with(".json") {
            serde_json::to_string_pretty(self)?
        } else if path.ends_with(".toml") {
            toml::to_string_pretty(self)?
        } else {
            return Err(anyhow::anyhow!("Unsupported config file format. Use .json or .toml"));
        };
        std::fs::write(path, config_str)?;
        Ok(())
    }

    fn generate_secret() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        
        (0..64)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub fn validate(&self) -> Result<()> {
        if self.adapter_port == self.client_port {
            return Err(anyhow::anyhow!("Adapter and client ports cannot be the same"));
        }

        if self.adapter_port < 1024 || self.client_port < 1024 {
            return Err(anyhow::anyhow!("Ports must be 1024 or higher"));
        }

        if self.adapter_secret.len() < 32 {
            return Err(anyhow::anyhow!("Adapter secret must be at least 32 characters"));
        }

        if self.client_secret.len() < 32 {
            return Err(anyhow::anyhow!("Client secret must be at least 32 characters"));
        }

        if self.adapter_secret == self.client_secret {
            return Err(anyhow::anyhow!("Adapter and client secrets must be different"));
        }

        if self.heartbeat_interval_seconds < 5 {
            return Err(anyhow::anyhow!("Heartbeat interval must be at least 5 seconds"));
        }

        if self.connection_timeout_seconds < 30 {
            return Err(anyhow::anyhow!("Connection timeout must be at least 30 seconds"));
        }

        // Validate rate limits
        if self.rate_limits.guest_requests_per_minute == 0 {
            return Err(anyhow::anyhow!("Guest rate limit must be greater than 0"));
        }

        if self.rate_limits.player_requests_per_minute < self.rate_limits.guest_requests_per_minute {
            return Err(anyhow::anyhow!("Player rate limit must be >= guest rate limit"));
        }

        if self.rate_limits.admin_requests_per_minute < self.rate_limits.player_requests_per_minute {
            return Err(anyhow::anyhow!("Admin rate limit must be >= player rate limit"));
        }

        if self.rate_limits.system_requests_per_minute < self.rate_limits.admin_requests_per_minute {
            return Err(anyhow::anyhow!("System rate limit must be >= admin rate limit"));
        }

        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => return Err(anyhow::anyhow!("Invalid log level. Use: trace, debug, info, warn, error")),
        }

        Ok(())
    }

    pub fn print_summary(&self) {
        println!("Bcon Server Configuration:");
        println!("  Adapter Port: {}", self.adapter_port);
        println!("  Client Port: {}", self.client_port);
        println!("  Log Level: {}", self.log_level);
        println!("  Server Name: {}", self.server_info.name);
        println!("  Heartbeat Interval: {}s", self.heartbeat_interval_seconds);
        println!("  Connection Timeout: {}s", self.connection_timeout_seconds);
        println!("  Rate Limits:");
        println!("    Guest: {}/min", self.rate_limits.guest_requests_per_minute);
        println!("    Player: {}/min", self.rate_limits.player_requests_per_minute);
        println!("    Admin: {}/min", self.rate_limits.admin_requests_per_minute);
        println!("    System: {}/min", self.rate_limits.system_requests_per_minute);
        println!("    Unauthenticated Adapter: {}/min", 
            self.rate_limits.unauthenticated_adapter_attempts_per_minute);
        println!("  Security:");
        println!("    Ban Threshold: {} violations", self.rate_limits.ban_threshold);
        println!("    Ban Duration: {} hours", self.rate_limits.ban_duration_hours);
    }

    pub fn create_example_config() -> Self {
        Self {
            adapter_port: 8082,
            client_port: 8081,
            adapter_secret: "your_secure_adapter_secret_here_at_least_32_chars_long".to_string(),
            client_secret: "your_secure_client_secret_here_at_least_32_chars_long".to_string(),
            rate_limits: RateLimitConfig {
                guest_requests_per_minute: 30,
                player_requests_per_minute: 120,
                admin_requests_per_minute: 300,
                system_requests_per_minute: 1000,
                unauthenticated_adapter_attempts_per_minute: 5,
                window_duration_seconds: 60,
                ban_threshold: 100,
                ban_duration_hours: 24,
            },
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "https://yourserver.com".to_string(),
            ],
            heartbeat_interval_seconds: 30,
            connection_timeout_seconds: 300,
            log_level: "info".to_string(),
            server_info: ServerInfo {
                name: "My Minecraft Server".to_string(),
                description: "A cool Minecraft server with web integration".to_string(),
                url: "play.myserver.com".to_string(),
                minecraft_version: "1.20.4".to_string(),
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native_config {
    use super::*;
    use clap::{Arg, Command};

    pub fn parse_cli_args() -> Result<Option<BconConfig>> {
        let matches = Command::new("bcon")
            .version("1.0.0")
            .about("Bcon WebSocket Communication Server")
            .arg(Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file"))
            .arg(Arg::new("adapter-port")
                .long("adapter-port")
                .value_name("PORT")
                .help("Port for adapter WebSocket server"))
            .arg(Arg::new("client-port")
                .long("client-port")
                .value_name("PORT")
                .help("Port for client WebSocket server"))
            .arg(Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info"))
            .arg(Arg::new("generate-config")
                .long("generate-config")
                .value_name("FILE")
                .help("Generate example config file and exit"))
            .get_matches();

        // Handle config generation
        if let Some(config_path) = matches.get_one::<String>("generate-config") {
            let example_config = BconConfig::create_example_config();
            example_config.save_to_file(config_path)?;
            println!("Example config saved to: {}", config_path);
            return Ok(None);
        }

        // Load base config
        let mut config = if let Some(config_path) = matches.get_one::<String>("config") {
            BconConfig::from_file(config_path)?
        } else {
            BconConfig::from_env()?
        };

        // Override with CLI args
        if let Some(port) = matches.get_one::<String>("adapter-port") {
            config.adapter_port = port.parse()?;
        }

        if let Some(port) = matches.get_one::<String>("client-port") {
            config.client_port = port.parse()?;
        }

        if let Some(level) = matches.get_one::<String>("log-level") {
            config.log_level = level.to_string();
        }

        config.validate()?;
        Ok(Some(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = BconConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port_config() {
        let mut config = BconConfig::default();
        config.adapter_port = config.client_port;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_secret_generation() {
        let secret1 = BconConfig::generate_secret();
        let secret2 = BconConfig::generate_secret();
        
        assert_ne!(secret1, secret2);
        assert_eq!(secret1.len(), 64);
        assert_eq!(secret2.len(), 64);
    }

    #[test]
    fn test_config_serialization() {
        let config = BconConfig::create_example_config();
        
        // Test JSON serialization
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: BconConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(config.adapter_port, deserialized.adapter_port);
        assert_eq!(config.client_port, deserialized.client_port);
    }

    #[test]
    fn test_secret_length_validation() {
        let mut config = BconConfig::default();
        config.adapter_secret = "short".to_string();
        assert!(config.validate().is_err());
        
        config.adapter_secret = "a".repeat(32);
        config.client_secret = "b".repeat(32);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_validation() {
        let mut config = BconConfig::default();
        config.rate_limits.guest_requests_per_minute = 0;
        assert!(config.validate().is_err());
        
        config.rate_limits.guest_requests_per_minute = 10;
        config.rate_limits.player_requests_per_minute = 5; // Less than guest
        assert!(config.validate().is_err());
    }
}