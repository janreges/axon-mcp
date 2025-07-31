use anyhow::{Context, Result};
use database::SqliteTaskRepository;
use mcp_protocol::McpServer;
use std::sync::Arc;
use tracing::info;

use crate::config::Config;

/// Create a task repository based on the complete configuration
pub async fn create_repository(config: &Config) -> Result<Arc<SqliteTaskRepository>> {
    info!("Creating task repository");
    
    // Get validated database URL from config (already handles defaults and validation)
    let database_url = config.database_url();
    info!("Using database URL: {}", database_url);

    // Create SQLite repository
    info!("Initializing SQLite repository at: {}", database_url);
    let repo = SqliteTaskRepository::new(&database_url)
        .await
        .context("Failed to create SQLite repository")?;

    // Run database migrations
    info!("Running database migrations");
    repo.migrate()
        .await
        .context("Failed to run database migrations")?;

    info!("Task repository created successfully");
    Ok(Arc::new(repo))
}

/// Create and configure the MCP server
pub fn create_server(repository: Arc<SqliteTaskRepository>, message_repository: Arc<SqliteTaskRepository>) -> Result<McpServer<SqliteTaskRepository, SqliteTaskRepository>> {
    info!("Creating MCP server");
    
    let server = McpServer::new(repository, message_repository);
    
    info!("MCP server created successfully");
    Ok(server)
}

/// Initialize the complete application
pub async fn initialize_app(config: &Config) -> Result<McpServer<SqliteTaskRepository, SqliteTaskRepository>> {
    info!("Initializing application");
    
    // Create repository
    let repository = create_repository(config)
        .await
        .context("Failed to create repository")?;
        
    // For now, use the same repository for both tasks and messages
    // In the future, these could be separate repositories
    let message_repository = repository.clone();
    
    // Create server
    let server = create_server(repository, message_repository)
        .context("Failed to create server")?;
    
    info!("Application initialized successfully");
    Ok(server)
}

/// Ensure the database directory exists using config
pub fn ensure_database_directory_from_config(config: &Config) -> Result<()> {
    let database_url = config.database_url();
    ensure_database_directory(&database_url)
}

/// Ensure the database directory exists
pub fn ensure_database_directory(database_url: &str) -> Result<()> {
    if let Some(db_path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            if !parent.exists() {
                info!("Creating database directory: {}", parent.display());
                std::fs::create_dir_all(parent)
                    .context("Failed to create database directory")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, ServerConfig, LoggingConfig, LogFormat};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_repository_with_default_url() {
        let config = Config {
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
        };

        let repo = create_repository(&config).await;
        match repo {
            Ok(_) => {}, // Test passes
            Err(e) => panic!("Failed to create repository: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_create_repository_with_custom_url() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());

        let config = Config {
            database: DatabaseConfig {
                url: Some(database_url),
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
        };

        let repo = create_repository(&config).await;
        assert!(repo.is_ok());
    }

    #[tokio::test]
    async fn test_create_repository_invalid_url() {
        let config = Config {
            database: DatabaseConfig {
                url: Some("postgres://invalid".to_string()),
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
        };

        let repo = create_repository(&config).await;
        assert!(repo.is_err());
    }

    #[test]
    fn test_ensure_database_directory() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("subdir").join("test.db");
        let database_url = format!("sqlite://{}", db_path.display());

        let result = ensure_database_directory(&database_url);
        assert!(result.is_ok());
        assert!(db_path.parent().unwrap().exists());
    }

    #[tokio::test]
    async fn test_create_server() {
        // Create a temporary database for testing
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("server_test.db");
        let database_url = format!("sqlite://{}", db_path.display());

        let config = Config {
            database: DatabaseConfig {
                url: Some(database_url),
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
        };

        let repo = create_repository(&config).await.unwrap();
        let message_repo = repo.clone();
        let server = create_server(repo, message_repo);
        assert!(server.is_ok());
    }
}