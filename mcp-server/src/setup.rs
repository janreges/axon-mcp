use anyhow::{Context, Result};
use database::{SqliteTaskRepository, SqliteWorkspaceContextRepository};
use mcp_protocol::McpServer;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use crate::config::{Config, resolve_database_path, InstallScope};

/// Create a task repository based on the complete configuration
pub async fn create_repository(config: &Config) -> Result<Arc<SqliteTaskRepository>> {
    info!("Creating task repository");

    // Handle legacy database migration before creating repository
    handle_legacy_database_migration()
        .context("Failed to handle legacy database migration")?;

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

/// Create a workspace context repository based on the complete configuration  
pub async fn create_workspace_context_repository(
    config: &Config,
) -> Result<Arc<SqliteWorkspaceContextRepository>> {
    info!("Creating workspace context repository");

    // Get validated database URL from config (already handles defaults and validation)
    let database_url = config.database_url();
    info!(
        "Using database URL for workspace contexts: {}",
        database_url
    );

    // Create repository with the database URL
    let repo = SqliteWorkspaceContextRepository::new(Arc::new(
        sqlx::SqlitePool::connect(&database_url)
            .await
            .with_context(|| {
                format!("Failed to connect to workspace context database at {database_url}")
            })?,
    ));

    info!("Workspace context repository created successfully");
    Ok(Arc::new(repo))
}

/// Create and configure the MCP server
pub fn create_server(
    repository: Arc<SqliteTaskRepository>,
    message_repository: Arc<SqliteTaskRepository>,
    workspace_context_repository: Arc<SqliteWorkspaceContextRepository>,
) -> Result<McpServer<SqliteTaskRepository, SqliteTaskRepository, SqliteWorkspaceContextRepository>>
{
    info!("Creating MCP server");

    let server = McpServer::new(repository, message_repository, workspace_context_repository);

    info!("MCP server created successfully");
    Ok(server)
}

/// Initialize the complete application
pub async fn initialize_app(
    config: &Config,
) -> Result<McpServer<SqliteTaskRepository, SqliteTaskRepository, SqliteWorkspaceContextRepository>>
{
    info!("Initializing application");

    // Create repository
    let repository = create_repository(config)
        .await
        .context("Failed to create repository")?;

    // For now, use the same repository for both tasks and messages
    // In the future, these could be separate repositories
    let message_repository = repository.clone();

    // Create workspace context repository
    let workspace_context_repository = create_workspace_context_repository(config)
        .await
        .context("Failed to create workspace context repository")?;

    // Create server
    let server = create_server(repository, message_repository, workspace_context_repository)
        .context("Failed to create server")?;

    info!("Application initialized successfully");
    Ok(server)
}

/// Ensure the database directory exists using config
pub fn ensure_database_directory_from_config(config: &Config) -> Result<()> {
    let database_url = config.database_url();
    ensure_database_directory(&database_url)
}

/// Ensure the database directory exists and set secure permissions
pub fn ensure_database_directory(database_url: &str) -> Result<()> {
    if let Some(db_path) = database_url.strip_prefix("sqlite://") {
        let db_path = Path::new(db_path);
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                info!("Creating database directory: {}", parent.display());
                std::fs::create_dir_all(parent).context("Failed to create database directory")?;
                
                // Set secure permissions on Unix systems (owner only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = std::fs::Permissions::from_mode(0o700);
                    std::fs::set_permissions(parent, permissions)
                        .context("Failed to set directory permissions")?;
                }
            }
        }
        
        // Set secure permissions on database file if it exists
        if db_path.exists() {
            set_secure_file_permissions(db_path)?;
        }
    }
    Ok(())
}

/// Set secure file permissions (owner-only access on Unix)
fn set_secure_file_permissions(file_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(file_path, permissions)
            .with_context(|| format!("Failed to set permissions for {}", file_path.display()))?;
        info!("Set secure permissions (0600) for database file: {}", file_path.display());
    }
    
    #[cfg(windows)]
    {
        // On Windows, we rely on NTFS permissions set by the parent directory
        // Future enhancement could use Windows ACL API for more granular control
        info!("Database file permissions managed by system on Windows: {}", file_path.display());
    }
    
    Ok(())
}

/// Handle migration from legacy database location
pub fn handle_legacy_database_migration() -> Result<()> {
    let (new_db_path, scope) = resolve_database_path()?;
    
    // Only attempt migration if we're in user scope and new DB doesn't exist
    if matches!(scope, InstallScope::User) && !new_db_path.exists() {
        let legacy_path = get_legacy_database_path();
        
        if legacy_path.exists() {
            info!("Legacy database found at: {}", legacy_path.display());
            info!("Migrating to new location: {}", new_db_path.display());
            
            // Ensure parent directory exists
            if let Some(parent) = new_db_path.parent() {
                std::fs::create_dir_all(parent)
                    .context("Failed to create migration directory")?;
            }
            
            // Copy legacy database to new location
            std::fs::copy(&legacy_path, &new_db_path)
                .with_context(|| format!(
                    "Failed to migrate database from {} to {}", 
                    legacy_path.display(), 
                    new_db_path.display()
                ))?;
            
            // Set secure permissions on the migrated database
            set_secure_file_permissions(&new_db_path)
                .context("Failed to set permissions on migrated database")?;
            
            // Create migration marker to avoid repeated attempts
            let marker_path = new_db_path.with_extension("migrated");
            std::fs::write(&marker_path, "Migrated from legacy location")
                .context("Failed to create migration marker")?;
            
            // Set secure permissions on marker file too
            set_secure_file_permissions(&marker_path)
                .context("Failed to set permissions on migration marker")?;
            
            info!("Database migration completed successfully");
            warn!("Legacy database file remains at {} for backup purposes", legacy_path.display());
            warn!("You may safely delete it after verifying the migration worked correctly");
        }
    }
    
    Ok(())
}

/// Get the legacy database path (~/axon-mcp.sqlite)
fn get_legacy_database_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("axon-mcp.sqlite")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, LogFormat, LoggingConfig, ServerConfig};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_repository_with_default_url() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let temp_db_path = temp_dir.path().join("test_default.db");

        let config = Config {
            database: DatabaseConfig {
                url: Some(format!("sqlite://{}", temp_db_path.display())),
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
            Ok(_) => {} // Test passes
            Err(e) => panic!("Failed to create repository: {e:?}"),
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
        let workspace_context_repo = create_workspace_context_repository(&config).await.unwrap();
        let server = create_server(repo, message_repo, workspace_context_repo);
        assert!(server.is_ok());
    }
}
