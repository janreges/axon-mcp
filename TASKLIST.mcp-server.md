# Task List: `mcp-server` Crate

**Owner Agent**: git-coordinator  
**Purpose**: Assemble all components into a running MCP server binary with configuration management and dependency injection.

## Critical Requirements

This crate MUST:
- Be the ONLY binary crate in the workspace
- Wire together all other crates correctly
- Handle configuration from environment and files
- Provide proper startup/shutdown handling
- Support SQLite backend with automatic database creation
- Include comprehensive logging and monitoring

## Phase 1: Project Setup ✓ Required

- [ ] Create `mcp-server/` directory
- [ ] Create `mcp-server/Cargo.toml` with dependencies:
  ```toml
  [package]
  name = "mcp-server"
  version = "0.1.0"
  edition = "2021"

  [[bin]]
  name = "mcp-server"
  path = "src/main.rs"

  [dependencies]
  task-core = { path = "../core" }
  database = { path = "../database" }
  mcp-protocol = { path = "../mcp-protocol" }
  
  tokio = { version = "1.0", features = ["full"] }
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  serde = { version = "1.0", features = ["derive"] }
  config = "0.13"
  dotenv = "0.15"
  anyhow = "1.0"
  clap = { version = "4.0", features = ["derive", "env"] }

  [dev-dependencies]
  mocks = { path = "../mocks" }
  ```
- [ ] Create directory structure:
  ```
  mcp-server/
  ├── src/
  │   ├── main.rs
  │   ├── config.rs
  │   ├── setup.rs
  │   └── telemetry.rs
  ├── config/
  │   ├── default.toml
  │   └── production.toml
  └── tests/
  ```

## Phase 2: Configuration Management ✓ Required

### Task 1: Create Configuration Structure (`src/config.rs`)
- [ ] Define `Config` struct:
  ```rust
  #[derive(Debug, Deserialize, Clone)]
  pub struct Config {
      pub database: DatabaseConfig,
      pub server: ServerConfig,
      pub logging: LoggingConfig,
  }
  
  #[derive(Debug, Deserialize, Clone)]
  pub struct DatabaseConfig {
      pub url: Option<String>,  // Optional, defaults to ~/db.sqlite
      pub max_connections: u32,
      pub connection_timeout: u64,
  }
  
  #[derive(Debug, Deserialize, Clone)]
  pub struct ServerConfig {
      pub listen_addr: String,
      pub port: u16,
      pub workers: usize,
  }
  
  #[derive(Debug, Deserialize, Clone)]
  pub struct LoggingConfig {
      pub level: String,
      pub format: LogFormat,
  }
  ```
- [ ] Implement configuration loading:
  ```rust
  impl Config {
      pub fn from_env() -> Result<Self>
      pub fn from_file(path: &str) -> Result<Self>
      pub fn merge_with_env(self) -> Result<Self>
  }
  ```
- [ ] Support environment variable overrides:
  - `DATABASE_URL`
  - `LISTEN_ADDR`
  - `LOG_LEVEL`

### Task 2: Create Default Configuration Files
- [ ] Create `config/default.toml`:
  ```toml
  [database]
  # url is optional, defaults to ~/db.sqlite
  max_connections = 5
  connection_timeout = 30

  [server]
  listen_addr = "127.0.0.1"
  port = 3000
  workers = 4

  [logging]
  level = "info"
  format = "pretty"
  ```
- [ ] Create `config/production.toml` with production defaults

## Phase 3: Application Setup ✓ Required

### Task 3: Create Setup Module (`src/setup.rs`)
- [ ] Create repository factory:
  ```rust
  pub async fn create_repository(config: &DatabaseConfig) -> Result<Arc<dyn TaskRepository>>
  ```
- [ ] Implement repository creation:
  ```rust
  // Get database URL with default fallback
  let database_url = config.database.url.unwrap_or_else(|| {
      let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
      format!("sqlite://{}/db.sqlite", home)
  });
  
  // Create SQLite repository
  let repo = SqliteTaskRepository::new(&database_url).await?;
  repo.migrate().await?;
  Ok(Arc::new(repo))
  ```
- [ ] Create server factory:
  ```rust
  pub fn create_server(repository: Arc<dyn TaskRepository>) -> McpServer<dyn TaskRepository>
  ```

### Task 4: Create Telemetry Module (`src/telemetry.rs`)
- [ ] Initialize tracing subscriber:
  ```rust
  pub fn init_telemetry(config: &LoggingConfig) -> Result<()>
  ```
- [ ] Configure log formatting (pretty/json)
- [ ] Set up log filtering by level
- [ ] Add span tracking for requests
- [ ] Include error reporting

### Task 5: Implement Main Entry Point (`src/main.rs`)
- [ ] Create CLI arguments:
  ```rust
  #[derive(Parser)]
  #[command(name = "mcp-server")]
  #[command(about = "MCP Task Management Server")]
  struct Cli {
      /// Configuration file path
      #[arg(short, long, env = "CONFIG_FILE")]
      config: Option<String>,
      
      /// Database URL override
      #[arg(long, env = "DATABASE_URL")]
      database_url: Option<String>,
      
      /// Listen address override
      #[arg(long, env = "LISTEN_ADDR")]
      listen_addr: Option<String>,
      
      /// Log level override
      #[arg(long, env = "LOG_LEVEL")]
      log_level: Option<String>,
  }
  ```
- [ ] Implement main function:
  ```rust
  #[tokio::main]
  async fn main() -> Result<()> {
      // Load .env file
      dotenv::dotenv().ok();
      
      // Parse CLI arguments
      let cli = Cli::parse();
      
      // Load configuration
      let config = load_config(&cli)?;
      
      // Initialize telemetry
      init_telemetry(&config.logging)?;
      
      // Create repository
      let repository = create_repository(&config.database).await?;
      
      // Create and start server
      let server = create_server(repository);
      let addr = format!("{}:{}", config.server.listen_addr, config.server.port);
      
      info!("Starting MCP server on {}", addr);
      server.serve(&addr).await?;
      
      Ok(())
  }
  ```

## Phase 4: Runtime Features ✓ Required

### Task 6: Add Graceful Shutdown
- [ ] Handle SIGTERM/SIGINT signals
- [ ] Implement shutdown coordinator
- [ ] Drain active SSE connections
- [ ] Close database connections properly
- [ ] Log shutdown progress

## Phase 5: Testing ✓ Required

### Task 7: Create Integration Tests
- [ ] Test server startup with SQLite
- [ ] Test configuration loading
- [ ] Test environment overrides
- [ ] Test default database path creation
- [ ] Test graceful shutdown
- [ ] Test error scenarios

### Task 8: Create End-to-End Tests
- [ ] Use mock repository for fast tests
- [ ] Test all MCP functions through SSE
- [ ] Test concurrent SSE clients
- [ ] Test SSE reconnection
- [ ] Test error handling over SSE

## Phase 6: Deployment Support ✓ Required

### Task 11: Create Docker Support
- [ ] Create Dockerfile:
  ```dockerfile
  FROM rust:1.75 as builder
  WORKDIR /app
  COPY . .
  RUN cargo build --release

  FROM debian:bookworm-slim
  RUN apt-get update && apt-get install -y ca-certificates
  COPY --from=builder /app/target/release/mcp-server /usr/local/bin/
  CMD ["mcp-server"]
  ```
- [ ] Add .dockerignore
- [ ] Create docker-compose.yml example

### Task 12: Create Systemd Service
- [ ] Create `mcp-server.service` file
- [ ] Add installation instructions
- [ ] Include log rotation config

## Public Interface Checklist ✓ Binary Requirements

### CLI Interface
- [ ] `--config` / `-c` - Config file path
- [ ] `--database-url` - Database URL override
- [ ] `--listen-addr` - Listen address override
- [ ] `--log-level` - Log level override
- [ ] `--help` / `-h` - Help message
- [ ] `--version` / `-V` - Version info

### Environment Variables
- [ ] `CONFIG_FILE` - Config file path
- [ ] `DATABASE_URL` - Database connection
- [ ] `LISTEN_ADDR` - Server address
- [ ] `LOG_LEVEL` - Logging level

### Exit Codes
- [ ] 0 - Clean shutdown
- [ ] 1 - Configuration error
- [ ] 2 - Database connection error
- [ ] 3 - Server startup error

## Quality Checklist

- [ ] Comprehensive error handling
- [ ] No panics in main code path
- [ ] All errors logged appropriately
- [ ] Resource cleanup on shutdown
- [ ] Configuration validation
- [ ] Security best practices
- [ ] Production-ready defaults

## Communication Points

Use `./log.sh` to communicate:
```bash
./log.sh "GIT-COORDINATOR → ALL: Integration starting, need all crates ready"
./log.sh "GIT-COORDINATOR → DEVOPS: Server binary ready for CI/CD setup"
./log.sh "GIT-COORDINATOR → QA-TESTER: Full system ready for E2E testing"
```

## Success Criteria

1. Server starts successfully with SQLite
2. All MCP functions accessible
3. Configuration system works properly
4. Graceful shutdown implemented
5. Comprehensive logging in place
6. Docker image builds and runs
7. Performance meets requirements
8. Zero crashes under normal operation