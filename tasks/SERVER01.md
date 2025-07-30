# SERVER01: Create MCP Server Application

## Objective
Create the main MCP server binary that assembles all components, provides the SSE endpoint, handles configuration, and serves as the primary entry point for the MCP v2 task management system.

## Implementation Details

### 1. Create Main Server Application
In `mcp-server/src/main.rs`:

```rust
use anyhow::Result;
use axum::{
    Router,
    middleware,
    http::{StatusCode, Method},
    response::IntoResponse,
    extract::State,
};
use clap::Parser;
use database::SqliteTaskRepository;
use mcp_protocol::{
    handler::McpProtocolHandler,
    transport::sse::create_mcp_router,
};
use std::{net::SocketAddr, sync::Arc, path::PathBuf};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    cors::{CorsLayer, Any},
    trace::{TraceLayer, DefaultOnRequest, DefaultOnResponse},
    compression::CompressionLayer,
};
use tracing::{info, error, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod health;
mod metrics;
mod startup;

use crate::config::ServerConfig;
use crate::startup::StartupManager;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    
    /// Override server port
    #[arg(short, long)]
    port: Option<u16>,
    
    /// Override database URL
    #[arg(short, long)]
    database_url: Option<String>,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    
    /// Run database migrations and exit
    #[arg(long)]
    migrate: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(args.debug);
    
    // Load configuration
    let mut config = ServerConfig::load(&args.config)?;
    
    // Apply command line overrides
    if let Some(port) = args.port {
        config.server.port = port;
    }
    
    if let Some(db_url) = args.database_url {
        config.database.url = db_url;
    }
    
    info!("Starting MCP v2 Server");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Config: {:?}", args.config);
    
    // Initialize database
    let db_url = config.database.url.clone();
    let pool = database::create_pool(&db_url).await?;
    
    // Run migrations if requested
    if args.migrate {
        info!("Running database migrations...");
        database::run_migrations(&pool).await?;
        info!("Migrations completed successfully");
        return Ok(());
    }
    
    // Create repository
    let repository = Arc::new(SqliteTaskRepository::new(pool.clone()));
    
    // Run startup tasks
    let startup_manager = StartupManager::new(repository.clone());
    startup_manager.run_startup_tasks(&config).await?;
    
    // Create MCP handler
    let handler = McpProtocolHandler::new(repository.clone());
    
    // Create routers
    let mcp_router = create_mcp_router(repository.clone());
    let health_router = health::create_health_router(pool.clone());
    let metrics_router = metrics::create_metrics_router(repository.clone());
    
    // Build main application
    let app = Router::new()
        .nest("/mcp/v2", mcp_router)
        .nest("/health", health_router)
        .nest("/metrics", metrics_router)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([Method::GET, Method::POST])
                        .allow_headers(Any)
                )
        );
    
    // Start background tasks
    start_background_tasks(repository.clone(), &config);
    
    // Bind to address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("MCP server listening on {}", addr);
    info!("SSE endpoint: http://{}/mcp/v2/sse", addr);
    info!("RPC endpoint: http://{}/mcp/v2/rpc", addr);
    
    // Run server with graceful shutdown
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    info!("Server shutdown complete");
    Ok(())
}

fn init_logging(debug: bool) {
    let env_filter = if debug {
        "debug,hyper=info,tower=info"
    } else {
        "info,mcp_server=debug"
    };
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| env_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}

fn start_background_tasks(repository: Arc<SqliteTaskRepository>, config: &ServerConfig) {
    let repo_clone = repository.clone();
    let check_interval = config.monitoring.health_check_interval_seconds;
    
    // Agent health monitoring
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(check_interval)
        );
        
        loop {
            interval.tick().await;
            
            if let Err(e) = check_agent_health(&repo_clone).await {
                error!("Agent health check failed: {}", e);
            }
        }
    });
    
    // Help request escalation
    let repo_clone = repository.clone();
    let escalation_interval = config.monitoring.escalation_check_interval_seconds;
    
    tokio::spawn(async move {
        database::help_escalation::auto_escalate_help_requests(
            repo_clone,
            escalation_interval,
        ).await;
    });
    
    // Metrics collection
    let repo_clone = repository.clone();
    let metrics_interval = config.monitoring.metrics_collection_interval_seconds;
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(metrics_interval)
        );
        
        loop {
            interval.tick().await;
            
            if let Err(e) = collect_system_metrics(&repo_clone).await {
                error!("Metrics collection failed: {}", e);
            }
        }
    });
}

async fn check_agent_health(repository: &SqliteTaskRepository) -> Result<()> {
    // Implementation would check for agents that haven't sent heartbeats
    // and mark them as offline
    Ok(())
}

async fn collect_system_metrics(repository: &SqliteTaskRepository) -> Result<()> {
    // Implementation would collect and store system metrics
    Ok(())
}
```

### 2. Create Configuration Module
In `mcp-server/src/config.rs`:

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub mcp: McpSettings,
    pub monitoring: MonitoringSettings,
    pub security: SecuritySettings,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub max_connections: usize,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpSettings {
    pub sse_keepalive_seconds: u64,
    pub max_message_size_bytes: usize,
    pub work_discovery_timeout_seconds: u64,
    pub work_discovery_poll_interval_seconds: u64,
    pub max_concurrent_requests_per_client: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringSettings {
    pub enable_metrics: bool,
    pub metrics_port: u16,
    pub health_check_interval_seconds: u64,
    pub escalation_check_interval_seconds: u64,
    pub metrics_collection_interval_seconds: u64,
    pub agent_heartbeat_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecuritySettings {
    pub enable_auth: bool,
    pub api_key_header: String,
    pub allowed_origins: Vec<String>,
    pub rate_limit_requests_per_minute: u32,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: None,
                max_connections: 1000,
                request_timeout_seconds: 30,
            },
            database: DatabaseSettings {
                url: "sqlite://~/mcp_tasks.db".to_string(),
                max_connections: 10,
                min_connections: 2,
                connect_timeout_seconds: 5,
                idle_timeout_seconds: 600,
            },
            mcp: McpSettings {
                sse_keepalive_seconds: 30,
                max_message_size_bytes: 1_048_576, // 1MB
                work_discovery_timeout_seconds: 120,
                work_discovery_poll_interval_seconds: 3,
                max_concurrent_requests_per_client: 10,
            },
            monitoring: MonitoringSettings {
                enable_metrics: true,
                metrics_port: 9090,
                health_check_interval_seconds: 60,
                escalation_check_interval_seconds: 300,
                metrics_collection_interval_seconds: 60,
                agent_heartbeat_timeout_seconds: 90,
            },
            security: SecuritySettings {
                enable_auth: false,
                api_key_header: "X-API-Key".to_string(),
                allowed_origins: vec!["*".to_string()],
                rate_limit_requests_per_minute: 600,
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
        }
    }
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let contents = fs::read_to_string(path)?;
            let config: ServerConfig = toml::from_str(&contents)?;
            Ok(config)
        } else {
            // Create default config
            let config = Self::default();
            let toml = toml::to_string_pretty(&config)?;
            fs::write(path, toml)?;
            Ok(config)
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            anyhow::bail!("Server port must be non-zero");
        }
        
        if self.database.max_connections < self.database.min_connections {
            anyhow::bail!("Max connections must be >= min connections");
        }
        
        if self.security.enable_tls {
            if self.security.tls_cert_path.is_none() || self.security.tls_key_path.is_none() {
                anyhow::bail!("TLS cert and key paths required when TLS is enabled");
            }
        }
        
        Ok(())
    }
}
```

### 3. Create Health Check Module
In `mcp-server/src/health.rs`:

```rust
use axum::{
    Router,
    routing::get,
    response::Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

pub fn create_health_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/live", get(liveness_check))
        .with_state(Arc::new(pool))
}

async fn health_check(State(pool): State<Arc<SqlitePool>>) -> Json<HealthStatus> {
    let start_time = std::time::Instant::now();
    let mut checks = Vec::new();
    
    // Database check
    let db_check = match check_database(&pool).await {
        Ok(_) => HealthCheck {
            name: "database".to_string(),
            status: "healthy".to_string(),
            message: None,
        },
        Err(e) => HealthCheck {
            name: "database".to_string(),
            status: "unhealthy".to_string(),
            message: Some(e.to_string()),
        },
    };
    checks.push(db_check);
    
    // Memory check
    let memory_check = check_memory();
    checks.push(memory_check);
    
    let all_healthy = checks.iter().all(|c| c.status == "healthy");
    
    Json(HealthStatus {
        status: if all_healthy { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime_seconds(),
        checks,
    })
}

async fn readiness_check(State(pool): State<Arc<SqlitePool>>) -> impl IntoResponse {
    match check_database(&pool).await {
        Ok(_) => (StatusCode::OK, "ready"),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "not ready"),
    }
}

async fn liveness_check() -> impl IntoResponse {
    (StatusCode::OK, "alive")
}

async fn check_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

fn check_memory() -> HealthCheck {
    use sysinfo::{System, SystemExt};
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;
    
    if usage_percent > 90.0 {
        HealthCheck {
            name: "memory".to_string(),
            status: "warning".to_string(),
            message: Some(format!("Memory usage: {:.1}%", usage_percent)),
        }
    } else {
        HealthCheck {
            name: "memory".to_string(),
            status: "healthy".to_string(),
            message: Some(format!("Memory usage: {:.1}%", usage_percent)),
        }
    }
}

fn get_uptime_seconds() -> u64 {
    // This would track actual uptime
    // For now, return a placeholder
    0
}
```

### 4. Create Metrics Module
In `mcp-server/src/metrics.rs`:

```rust
use axum::{
    Router,
    routing::get,
    response::{IntoResponse, Response},
    extract::State,
};
use prometheus::{Encoder, TextEncoder, Counter, Gauge, Histogram, Registry};
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    static ref HTTP_REQUESTS_TOTAL: Counter = Counter::new(
        "http_requests_total", "Total number of HTTP requests"
    ).expect("metric can be created");
    
    static ref HTTP_REQUEST_DURATION: Histogram = Histogram::with_opts(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request latency"
        )
    ).expect("metric can be created");
    
    static ref ACTIVE_AGENTS: Gauge = Gauge::new(
        "mcp_active_agents", "Number of active agents"
    ).expect("metric can be created");
    
    static ref TASKS_IN_QUEUE: Gauge = Gauge::new(
        "mcp_tasks_in_queue", "Number of tasks in queue"
    ).expect("metric can be created");
    
    static ref TASKS_COMPLETED_TOTAL: Counter = Counter::new(
        "mcp_tasks_completed_total", "Total number of completed tasks"
    ).expect("metric can be created");
}

pub fn init_metrics() {
    REGISTRY
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .expect("collector can be registered");
    
    REGISTRY
        .register(Box::new(HTTP_REQUEST_DURATION.clone()))
        .expect("collector can be registered");
    
    REGISTRY
        .register(Box::new(ACTIVE_AGENTS.clone()))
        .expect("collector can be registered");
    
    REGISTRY
        .register(Box::new(TASKS_IN_QUEUE.clone()))
        .expect("collector can be registered");
    
    REGISTRY
        .register(Box::new(TASKS_COMPLETED_TOTAL.clone()))
        .expect("collector can be registered");
}

pub fn create_metrics_router<R: TaskRepository + 'static>(repository: Arc<R>) -> Router {
    init_metrics();
    
    Router::new()
        .route("/", get(metrics_handler))
        .with_state(repository)
}

async fn metrics_handler<R: TaskRepository>(
    State(repository): State<Arc<R>>
) -> Response {
    // Update dynamic metrics
    if let Ok(agents) = repository.list_agents(AgentFilter::default()).await {
        let active_count = agents.iter().filter(|a| a.is_available()).count();
        ACTIVE_AGENTS.set(active_count as f64);
    }
    
    if let Ok(queue_size) = repository.count_tasks_by_state(
        TaskState::Created,
        None,
        None
    ).await {
        TASKS_IN_QUEUE.set(queue_size as f64);
    }
    
    // Encode metrics
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Response::builder()
        .status(200)
        .header("Content-Type", encoder.format_type())
        .body(buffer.into())
        .unwrap()
}

// Middleware to track HTTP metrics
pub async fn track_metrics<B>(
    req: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> Response {
    let start = std::time::Instant::now();
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    
    HTTP_REQUESTS_TOTAL.inc();
    
    let response = next.run(req).await;
    
    let duration = start.elapsed().as_secs_f64();
    HTTP_REQUEST_DURATION.observe(duration);
    
    response
}
```

### 5. Create Startup Manager
In `mcp-server/src/startup.rs`:

```rust
use anyhow::Result;
use core::{models::*, repository::TaskRepository};
use std::sync::Arc;
use tracing::{info, warn};

pub struct StartupManager<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> StartupManager<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    pub async fn run_startup_tasks(&self, config: &ServerConfig) -> Result<()> {
        info!("Running startup tasks...");
        
        // Check database connectivity
        self.verify_database().await?;
        
        // Initialize system agent if not exists
        self.ensure_system_agent().await?;
        
        // Clean up stale data
        self.cleanup_stale_data().await?;
        
        // Restore agent states
        self.restore_agent_states().await?;
        
        // Log system stats
        self.log_system_stats().await?;
        
        info!("Startup tasks completed");
        Ok(())
    }
    
    async fn verify_database(&self) -> Result<()> {
        info!("Verifying database connectivity...");
        
        // Try to count tasks as a simple query
        match self.repository.count_tasks(None, None).await {
            Ok(count) => {
                info!("Database connected. Total tasks: {}", count);
                Ok(())
            }
            Err(e) => {
                anyhow::bail!("Database verification failed: {}", e);
            }
        }
    }
    
    async fn ensure_system_agent(&self) -> Result<()> {
        let system_agent_name = "system";
        
        match self.repository.get_agent(system_agent_name).await? {
            Some(_) => {
                info!("System agent already exists");
            }
            None => {
                info!("Creating system agent...");
                
                let system_agent = NewAgentProfile {
                    name: system_agent_name.to_string(),
                    capabilities: vec!["system".to_string(), "monitoring".to_string()],
                    specializations: vec!["system".to_string()],
                    max_concurrent_tasks: 100,
                    description: Some("System agent for internal operations".to_string()),
                    ..Default::default()
                };
                
                self.repository.register_agent(system_agent).await?;
                info!("System agent created");
            }
        }
        
        Ok(())
    }
    
    async fn cleanup_stale_data(&self) -> Result<()> {
        info!("Cleaning up stale data...");
        
        // Mark agents as offline if they haven't sent heartbeat
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        let cutoff = Utc::now() - Duration::minutes(5);
        
        for agent in agents {
            if agent.last_heartbeat < cutoff && agent.status != AgentStatus::Offline {
                warn!("Marking agent {} as offline (last heartbeat: {})", 
                    agent.name, agent.last_heartbeat);
                
                self.repository.update_agent_status(
                    &agent.name,
                    AgentStatus::Offline
                ).await?;
            }
        }
        
        // Clean up very old completed tasks (optional)
        // This would depend on retention policy
        
        Ok(())
    }
    
    async fn restore_agent_states(&self) -> Result<()> {
        info!("Restoring agent states...");
        
        // Reset all agents to offline on startup
        // They will come back online when they send heartbeats
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        
        for agent in agents {
            if agent.name != "system" && agent.status != AgentStatus::Offline {
                self.repository.update_agent_status(
                    &agent.name,
                    AgentStatus::Offline
                ).await?;
            }
        }
        
        info!("Reset {} agents to offline state", agents.len() - 1);
        Ok(())
    }
    
    async fn log_system_stats(&self) -> Result<()> {
        let total_tasks = self.repository.count_tasks(None, None).await?;
        let active_tasks = self.repository.count_tasks_by_state(
            TaskState::InProgress,
            None,
            None
        ).await?;
        let queued_tasks = self.repository.count_tasks_by_state(
            TaskState::Created,
            None,
            None
        ).await?;
        
        let agents = self.repository.list_agents(AgentFilter::default()).await?;
        
        info!("System Statistics:");
        info!("  Total tasks: {}", total_tasks);
        info!("  Active tasks: {}", active_tasks);
        info!("  Queued tasks: {}", queued_tasks);
        info!("  Registered agents: {}", agents.len());
        
        Ok(())
    }
}
```

### 6. Create Docker Configuration
In `mcp-server/Dockerfile`:

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY core ./core
COPY database ./database
COPY mcp-protocol ./mcp-protocol
COPY mcp-server ./mcp-server

# Build release binary
RUN cargo build --release --bin mcp-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 mcp

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/mcp-server /app/mcp-server

# Copy default config
COPY mcp-server/config.default.toml /app/config.toml

# Set ownership
RUN chown -R mcp:mcp /app

USER mcp

# Environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:///app/data/mcp_tasks.db

# Create data directory
RUN mkdir -p /app/data

EXPOSE 8080 9090

ENTRYPOINT ["/app/mcp-server"]
```

### 7. Create Systemd Service
In `mcp-server/mcp-server.service`:

```ini
[Unit]
Description=MCP v2 Task Management Server
After=network.target

[Service]
Type=simple
User=mcp
Group=mcp
WorkingDirectory=/opt/mcp-server
ExecStart=/opt/mcp-server/mcp-server --config /etc/mcp-server/config.toml
Restart=always
RestartSec=5

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/mcp-server

# Resource limits
LimitNOFILE=65535
LimitNPROC=4096

# Environment
Environment="RUST_LOG=info"
Environment="DATABASE_URL=sqlite:///var/lib/mcp-server/tasks.db"

[Install]
WantedBy=multi-user.target
```

## Files to Create
- `mcp-server/src/main.rs` - Main server application
- `mcp-server/src/config.rs` - Configuration management
- `mcp-server/src/health.rs` - Health check endpoints
- `mcp-server/src/metrics.rs` - Prometheus metrics
- `mcp-server/src/startup.rs` - Startup tasks
- `mcp-server/Dockerfile` - Docker image
- `mcp-server/config.default.toml` - Default configuration
- `mcp-server/mcp-server.service` - Systemd service

## Dependencies
```toml
[dependencies]
core = { path = "../core" }
database = { path = "../database" }
mcp-protocol = { path = "../mcp-protocol" }

anyhow = "1.0"
axum = { version = "0.7", features = ["macros", "ws"] }
clap = { version = "4.0", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
prometheus = "0.13"
lazy_static = "1.4"
sysinfo = "0.30"
```

## Running the Server

### Development
```bash
cargo run --bin mcp-server -- --debug
```

### Production
```bash
./mcp-server --config /etc/mcp-server/config.toml
```

### Docker
```bash
docker build -t mcp-server .
docker run -p 8080:8080 -p 9090:9090 -v $(pwd)/data:/app/data mcp-server
```

## Notes
- SSE endpoint for real-time updates
- Health checks for monitoring
- Prometheus metrics endpoint
- Graceful shutdown handling
- Background task management
- Configurable via TOML or environment variables