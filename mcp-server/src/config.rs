use anyhow::{Context, Result};
use config::{Config as ConfigBuilder, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};

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

        let config = builder.build().context("Failed to build configuration")?;

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

    /// Get the default database URL using dynamic path resolution
    pub fn default_database_url() -> String {
        // Use dynamic path resolution based on installation scope
        match resolve_database_url() {
            Ok(url) => url,
            Err(_) => {
                // Fallback to legacy behavior if dynamic resolution fails
                let home = env::var("HOME")
                    .or_else(|_| env::var("USERPROFILE"))
                    .unwrap_or_else(|_| ".".to_string());
                format!("sqlite://{home}/axon-mcp.sqlite")
            }
        }
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

/// Installation scope detection for dynamic database path resolution
#[derive(Debug, Clone, PartialEq)]
pub enum InstallScope {
    /// Project-specific installation - database in .axon directory
    Project,
    /// User-global installation - database in user data directory with project hash
    User,
    /// Explicit override via environment variable or CLI
    Override,
}

/// Detect the installation scope based on execution context
pub fn detect_installation_scope() -> InstallScope {
    // 1. Check for explicit scope override via environment variable
    if let Ok(scope_env) = env::var("AXON_MCP_SCOPE") {
        return match scope_env.to_lowercase().as_str() {
            "project" => InstallScope::Project,
            "user" => InstallScope::User,
            _ => InstallScope::User, // Default fallback
        };
    }

    // 2. Executable path heuristic - if executable is inside project root, it's project-scoped
    if let (Ok(current_dir), Ok(executable_path)) = (env::current_dir(), env::current_exe()) {
        // Look for software project root (Cargo.toml, .git, package.json, etc.)
        if let Some(project_root) = find_project_root(&current_dir) {
            // If executable is within the project directory, it's project-scoped
            if executable_path.starts_with(&project_root) {
                return InstallScope::Project;
            }
        }

        // 3. Check for explicit Axon MCP project marker
        if has_axon_marker(&current_dir) {
            return InstallScope::Project;
        }
    }

    // 4. Default to user-scope for global installations
    InstallScope::User
}

/// Find the root of a software project by looking for common marker files
fn find_project_root(start_path: &Path) -> Option<PathBuf> {
    let markers = [
        "Cargo.toml",    // Rust
        ".git",          // Git repository
        "package.json",  // Node.js
        "pyproject.toml", // Python
        "go.mod",        // Go
        "Gemfile",       // Ruby
        "pom.xml",       // Java/Maven
        "build.gradle",  // Java/Gradle
        ".project",      // Eclipse
        "composer.json", // PHP
    ];

    let mut current = start_path;
    loop {
        for marker in &markers {
            if current.join(marker).exists() {
                return Some(current.to_path_buf());
            }
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    None
}

/// Check if the current directory has an explicit Axon MCP project marker
fn has_axon_marker(path: &Path) -> bool {
    path.join(".axon-mcp.toml").exists()
}

/// Resolve the database path based on scope and context
pub fn resolve_database_path() -> Result<(PathBuf, InstallScope)> {
    // 1. Check for explicit database path override (highest priority)
    if let Ok(db_path) = env::var("AXON_MCP_DB") {
        return Ok((PathBuf::from(db_path), InstallScope::Override));
    }

    // 2. Detect installation scope
    let scope = detect_installation_scope();
    let current_dir = env::current_dir().context("Failed to get current directory")?;

    let db_path = match scope {
        InstallScope::Project => {
            // Project-scoped: store database in .axon directory within project
            let project_root = find_project_root(&current_dir).unwrap_or(current_dir);
            project_root.join(".axon").join("axon-mcp.sqlite")
        }
        InstallScope::User => {
            // User-scoped: store database in user data directory with project hash
            let data_dir = dirs::data_dir()
                .context("Failed to determine user data directory")?
                .join("axon-mcp")
                .join("dbs");

            // Create a stable hash of the project root path
            let project_root = find_project_root(&current_dir).unwrap_or(current_dir);
            let canonical_root = project_root.canonicalize().unwrap_or(project_root);
            let project_path_str = canonical_root.to_string_lossy();

            let mut hasher = Sha256::new();
            hasher.update(project_path_str.as_bytes());
            let hash_result = hasher.finalize();
            let hash_hex = hex::encode(hash_result);

            // Use first 12 characters of hash for shorter filename
            data_dir.join(format!("{}.sqlite", &hash_hex[..12]))
        }
        InstallScope::Override => {
            // This should not happen as we check for override at the beginning
            return Err(anyhow::anyhow!("Unexpected override scope in path resolution"));
        }
    };

    Ok((db_path, scope))
}

/// Get the database URL with dynamic path resolution
pub fn resolve_database_url() -> Result<String> {
    let (db_path, _scope) = resolve_database_path()?;
    Ok(format!("sqlite://{}", db_path.display()))
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
        // URL now uses dynamic path resolution - it could be project or user scope
        // In user scope, it uses a hash, so just check it's valid SQLite
        assert!(url.contains(".sqlite"));
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

    #[test]
    fn test_detect_installation_scope_env_override() {
        // Test explicit scope override
        env::set_var("AXON_MCP_SCOPE", "project");
        assert_eq!(detect_installation_scope(), InstallScope::Project);

        env::set_var("AXON_MCP_SCOPE", "user");
        assert_eq!(detect_installation_scope(), InstallScope::User);

        env::remove_var("AXON_MCP_SCOPE");
    }

    #[test]
    fn test_resolve_database_path_with_override() {
        env::set_var("AXON_MCP_DB", "/custom/path/db.sqlite");
        
        let (path, scope) = resolve_database_path().unwrap();
        assert_eq!(path, PathBuf::from("/custom/path/db.sqlite"));
        assert_eq!(scope, InstallScope::Override);

        env::remove_var("AXON_MCP_DB");
    }

    #[test]
    fn test_find_project_root() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().join("project");
        let nested_dir = project_dir.join("src").join("deep");
        std::fs::create_dir_all(&nested_dir).unwrap();

        // Create a Cargo.toml marker
        std::fs::write(project_dir.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        // Test finding project root from nested directory
        let found_root = find_project_root(&nested_dir);
        assert_eq!(found_root, Some(project_dir));

        // Test when no project root is found
        let no_root = find_project_root(&temp_dir.path().join("no-project"));
        assert_eq!(no_root, None);
    }

    #[test]
    fn test_has_axon_marker() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        
        // Initially no marker
        assert!(!has_axon_marker(temp_dir.path()));

        // Create marker file
        std::fs::write(temp_dir.path().join(".axon-mcp.toml"), "").unwrap();
        assert!(has_axon_marker(temp_dir.path()));
    }

    #[test]
    fn test_resolve_database_url() {
        // Test that resolve_database_url returns a valid SQLite URL
        let url = resolve_database_url().unwrap();
        assert!(url.starts_with("sqlite://"));
    }
}
