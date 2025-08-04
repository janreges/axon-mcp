use mcp_server::config::{Config, DatabaseConfig, LogFormat, LoggingConfig, ServerConfig};
use mcp_server::setup::{create_repository, ensure_database_directory};
use std::env;
use tempfile::TempDir;

#[tokio::test]
async fn test_server_startup_with_sqlite() {
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
        project: mcp_server::config::ProjectConfig {
            root: None,
        },
    };

    let repo = create_repository(&config).await;
    assert!(
        repo.is_ok(),
        "Failed to create repository: {:?}",
        repo.err()
    );
}

#[test]
fn test_configuration_loading() {
    // Test default configuration
    let config = Config::default();
    assert!(config.validate().is_ok());
    assert_eq!(config.server.listen_addr, "127.0.0.1");
    assert_eq!(config.server.port, 3000);
}

#[test]
fn test_environment_overrides() {
    // Set environment variables
    env::set_var("DATABASE_URL", "sqlite://test_env.db");
    env::set_var("LISTEN_ADDR", "0.0.0.0");
    env::set_var("LOG_LEVEL", "debug");

    let config = Config::default().merge_with_env().unwrap();

    assert_eq!(
        config.database.url,
        Some("sqlite://test_env.db".to_string())
    );
    assert_eq!(config.server.listen_addr, "0.0.0.0");
    assert_eq!(config.logging.level, "debug");

    // Clean up
    env::remove_var("DATABASE_URL");
    env::remove_var("LISTEN_ADDR");
    env::remove_var("LOG_LEVEL");
}

#[test]
fn test_default_database_path_creation() {
    let config = Config::default();
    let url = config.database_url();

    println!("Generated URL: {}", url); // Debug output
    assert!(url.starts_with("sqlite://"));
    // URL now uses dynamic path resolution - in user scope it uses hash
    assert!(url.contains(".sqlite"));
}

#[test]
fn test_database_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("subdir").join("test.db");
    let database_url = format!("sqlite://{}", db_path.display());

    let result = ensure_database_directory(&database_url);
    assert!(result.is_ok());
    assert!(db_path.parent().unwrap().exists());
}

#[test]
fn test_config_validation_errors() {
    let mut config = Config::default();

    // Test invalid log level
    config.logging.level = "invalid".to_string();
    assert!(config.validate().is_err());

    // Test invalid database URL
    config.logging.level = "info".to_string();
    config.database.url = Some("postgres://invalid".to_string());
    assert!(config.validate().is_err());

    // Test invalid port
    config.database.url = None;
    config.server.port = 0;
    assert!(config.validate().is_err());

    // Test invalid workers
    config.server.port = 3000;
    config.server.workers = 0;
    assert!(config.validate().is_err());

    // Test invalid max_connections
    config.server.workers = 4;
    config.database.max_connections = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_server_address_formatting() {
    let config = Config {
        database: DatabaseConfig {
            url: None,
            max_connections: 5,
            connection_timeout: 30,
        },
        server: ServerConfig {
            listen_addr: "0.0.0.0".to_string(),
            port: 8080,
            workers: 2,
        },
        logging: LoggingConfig {
            level: "info".to_string(),
            format: LogFormat::Json,
        },
        project: mcp_server::config::ProjectConfig {
            root: None,
        },
    };

    assert_eq!(config.server_address(), "0.0.0.0:8080");
}

#[tokio::test]
async fn test_repository_creation_with_migrations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("migration_test.db");
    let database_url = format!("sqlite://{}", db_path.display());

    let config = Config {
        database: DatabaseConfig {
            url: Some(database_url),
            max_connections: 3,
            connection_timeout: 15,
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
        project: mcp_server::config::ProjectConfig {
            root: None,
        },
    };

    let repo = create_repository(&config).await;
    assert!(repo.is_ok());

    // Verify the database file was created
    assert!(db_path.exists());
}

#[tokio::test]
async fn test_multiple_repository_instances() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("multi_test.db");
    let database_url = format!("sqlite://{}", db_path.display());

    let config = Config {
        database: DatabaseConfig {
            url: Some(database_url),
            max_connections: 10,
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
        project: mcp_server::config::ProjectConfig {
            root: None,
        },
    };

    // Create multiple repository instances
    let repo1 = create_repository(&config).await;
    let repo2 = create_repository(&config).await;

    assert!(repo1.is_ok());
    assert!(repo2.is_ok());
}
