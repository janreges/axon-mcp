use anyhow::{Context, Result};
use config::{Config as ConfigBuilder, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    /// Optional database URL. If not provided, defaults to ~/db.sqlite
    pub url: Option<String>,
    /// Maximum number of database connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    /// Listen address for the MCP server
    pub listen_addr: String,
    /// Port number to listen on
    pub port: u16,
    /// Number of worker threads
    pub workers: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log format (pretty, json, compact)
    pub format: LogFormat,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Pretty,
    Json,
    Compact,
}

impl Config {
    /// Load configuration from environment variables and config files
    pub fn from_env() -> Result<Self> {
        let mut builder = ConfigBuilder::builder();

        // Start with default configuration
        builder = builder.add_source(File::from_str(
            include_str!("../config/default.toml"),
            FileFormat::Toml,
        ));

        // Add config file if specified
        if let Ok(config_file) = env::var("CONFIG_FILE") {
            builder = builder.add_source(
                File::with_name(&config_file)
                    .required(false)
                    .format(FileFormat::Toml),
            );
        }

        // Add environment variable overrides with MCP_ prefix
        builder = builder.add_source(
            Environment::with_prefix("MCP")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder
            .build()
            .context("Failed to build configuration")?;

        let mut result: Config = config
            .try_deserialize()
            .context("Failed to deserialize configuration")?;

        // Handle standard environment variables (DATABASE_URL, LISTEN_ADDR, LOG_LEVEL)
        // This provides compatibility while using the config crate as the primary source
        Self::apply_standard_env_vars(&mut result);

        Ok(result)
    }

    /// Load configuration from a specific file path
    pub fn from_file(path: &str) -> Result<Self> {
        let builder = ConfigBuilder::builder()
            .add_source(File::with_name(path).format(FileFormat::Toml))
            .add_source(
                Environment::with_prefix("MCP")
                    .separator("_")
                    .try_parsing(true),
            );

        let config = builder
            .build()
            .context("Failed to build configuration from file")?;

        config
            .try_deserialize()
            .context("Failed to deserialize configuration from file")
    }

    /// Apply standard environment variables (DATABASE_URL, LISTEN_ADDR, LOG_LEVEL)
    /// This provides compatibility with common deployment patterns
    fn apply_standard_env_vars(config: &mut Config) {
        if let Ok(database_url) = env::var("DATABASE_URL") {
            config.database.url = Some(database_url);
        }

        if let Ok(listen_addr) = env::var("LISTEN_ADDR") {
            config.server.listen_addr = listen_addr;
        }

        if let Ok(log_level) = env::var("LOG_LEVEL") {
            config.logging.level = log_level;
        }
    }

    /// Merge current configuration with environment variables using config crate
    #[allow(dead_code)]
    pub fn merge_with_env(mut self) -> Result<Self> {
        // Apply standard environment variables for compatibility
        Self::apply_standard_env_vars(&mut self);
        Ok(self)
    }

    /// Get the database URL with default fallback to ~/db.sqlite
    pub fn database_url(&self) -> String {
        match &self.database.url {
            Some(url) => url.clone(),
            None => Self::default_database_url(),
        }
    }

    /// Get the default database URL, with improved production support
    pub fn default_database_url() -> String {
        // Use XDG_DATA_HOME if available (better for containers/production)
        if let Ok(xdg_data) = env::var("XDG_DATA_HOME") {
            return format!("sqlite://{xdg_data}/axon-mcp/axon-mcp.sqlite");
        }

        // Fallback to HOME directory
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| {
                // Last resort: use current directory if no home is available
                ".".to_string()
            });
        format!("sqlite://{home}/axon-mcp.sqlite")
    }

    /// Get the server socket address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.listen_addr, self.server.port)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate log level
        match self.logging.level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {}
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
                    self.logging.level
                ));
            }
        }

        // Validate database URL format (both configured and default)
        let database_url = self.database_url();
        if !database_url.starts_with("sqlite://") {
            return Err(anyhow::anyhow!(
                "Only SQLite databases are supported. URL must start with 'sqlite://'. Got: {}",
                database_url
            ));
        }

        // Validate server configuration
        if self.server.port == 0 {
            return Err(anyhow::anyhow!("Server port cannot be 0"));
        }

        if self.server.workers == 0 {
            return Err(anyhow::anyhow!("Server workers must be greater than 0"));
        }

        if self.database.max_connections == 0 {
            return Err(anyhow::anyhow!(
                "Database max_connections must be greater than 0"
            ));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                url: None,
                max_connections: 5,
                connection_timeout: 30,
            },
            server: ServerConfig {
                listen_addr: "127.0.0.1".to_string(),
                port: 3000,
                workers: 4,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: LogFormat::Pretty,
            },
        }
    }
}

/// Helper function to get the default database path
#[allow(dead_code)]
pub fn default_database_path() -> PathBuf {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("db.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.listen_addr, "127.0.0.1");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.database.max_connections, 5);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_database_url_with_default() {
        let config = Config::default();
        let url = config.database_url();
        assert!(url.starts_with("sqlite://"));
        assert!(url.contains("db.sqlite"));
    }

    #[test]
    fn test_database_url_with_custom() {
        let mut config = Config::default();
        config.database.url = Some("sqlite://custom.db".to_string());
        assert_eq!(config.database_url(), "sqlite://custom.db");
    }

    #[test]
    fn test_server_address() {
        let config = Config::default();
        assert_eq!(config.server_address(), "127.0.0.1:3000");
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());

        let mut invalid_config = Config::default();
        invalid_config.logging.level = "invalid".to_string();
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_environment_override() {
        env::set_var("DATABASE_URL", "sqlite://test.db");
        let config = Config::default().merge_with_env().unwrap();
        assert_eq!(config.database.url, Some("sqlite://test.db".to_string()));
        env::remove_var("DATABASE_URL");
    }
}