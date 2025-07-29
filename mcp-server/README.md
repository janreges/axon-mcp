# MCP Server Binary

Production-ready binary for the MCP Task Management Server with comprehensive configuration management, telemetry, and operational features.

## Overview

The `mcp-server` crate provides the main executable that integrates all components of the MCP Task Management System into a production-ready server binary. It includes:

- **Configuration Management**: File-based and environment variable configuration
- **Database Setup**: Automatic SQLite database initialization and migrations
- **Telemetry**: Comprehensive logging, tracing, and metrics collection
- **Graceful Shutdown**: Signal handling for clean service termination
- **CLI Interface**: Command-line options for operations and debugging
- **Docker Support**: Container-ready with health checks

## Quick Start

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd task-manager

# Build the server
cargo build --release

# Run with default configuration
./target/release/mcp-server
```

The server will start with:
- Default configuration from `config/default.toml`
- SQLite database at `~/db.sqlite`
- Server listening on `127.0.0.1:3000`
- INFO level logging to stdout

### Configuration

#### Environment Variables

```bash
# Database configuration
export DATABASE_URL="sqlite:///path/to/database.sqlite"
export DATABASE_MAX_CONNECTIONS="20"

# Server configuration
export LISTEN_ADDR="0.0.0.0:3000"
export SERVER_TIMEOUT_SECONDS="30"

# Logging configuration  
export LOG_LEVEL="debug"
export LOG_FORMAT="json"

# Start server
./target/release/mcp-server
```

#### Configuration File

Create `config/production.toml`:

```toml
[server]
listen_addr = "0.0.0.0:3000"
timeout_seconds = 30
max_connections = 1000

[database]
url = "sqlite:///data/production.sqlite"
max_connections = 50
connection_timeout_seconds = 5

[logging]
level = "info"
format = "json"
directory = "/var/log/mcp-server"
```

Run with configuration file:

```bash
./target/release/mcp-server --config config/production.toml
```

#### CLI Options

```bash
./target/release/mcp-server --help
```

Available options:

```
MCP Task Management Server

Usage: mcp-server [OPTIONS]

Options:
  -c, --config <CONFIG>           Configuration file path
      --database-url <URL>        Database URL override  
      --listen-addr <ADDR>        Listen address override
      --log-level <LEVEL>         Log level override (trace, debug, info, warn, error)
  -h, --help                      Print help
  -V, --version                   Print version
```

## Configuration Reference

### Server Configuration

```toml
[server]
# Address and port to bind to
listen_addr = "127.0.0.1:3000"

# Request timeout in seconds
timeout_seconds = 30

# Maximum concurrent connections
max_connections = 1000

# Enable CORS for web clients
enable_cors = true

# Health check endpoint path  
health_check_path = "/health"

# Metrics endpoint path
metrics_path = "/metrics"
```

### Database Configuration

```toml
[database]
# SQLite database URL (supports file paths and :memory:)
url = "sqlite:///path/to/database.sqlite"

# Connection pool size
max_connections = 10
min_connections = 1

# Connection timeout
connection_timeout_seconds = 30

# Idle connection timeout
idle_timeout_seconds = 600

# Enable SQLite WAL mode for better concurrency
enable_wal_mode = true

# Enable foreign key constraints
enable_foreign_keys = true
```

### Logging Configuration

```toml
[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty, compact
format = "json"

# Log directory (optional, defaults to stdout)
directory = "/var/log/mcp-server"

# Maximum log file size in MB
max_file_size_mb = 100

# Maximum number of log files to keep
max_files = 10

# Enable ANSI colors in output
enable_colors = true
```

### Telemetry Configuration

```toml
[telemetry] 
# Enable OpenTelemetry tracing
enable_tracing = true

# Jaeger endpoint for trace export
jaeger_endpoint = "http://localhost:14268/api/traces"

# Prometheus metrics endpoint
metrics_endpoint = "http://localhost:9090/metrics"

# Service name for tracing
service_name = "mcp-task-server"

# Environment tag
environment = "production"
```

## Operations

### Running in Production

#### Systemd Service

Create `/etc/systemd/system/mcp-server.service`:

```ini
[Unit]
Description=MCP Task Management Server
After=network.target
Wants=network.target

[Service]
Type=simple
User=mcp-server
Group=mcp-server
WorkingDirectory=/opt/mcp-server
ExecStart=/opt/mcp-server/mcp-server --config /etc/mcp-server/production.toml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

# Security settings
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=/var/lib/mcp-server /var/log/mcp-server

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable mcp-server
sudo systemctl start mcp-server
sudo systemctl status mcp-server
```

#### Docker Deployment

Build Docker image:

```bash
docker build -t mcp-server .
```

Run container:

```bash
docker run -d \
  --name mcp-server \
  -p 3000:3000 \
  -v /data/mcp-server:/data \
  -e DATABASE_URL="sqlite:///data/tasks.sqlite" \
  -e LOG_LEVEL="info" \
  mcp-server
```

Docker Compose:

```yaml
version: '3.8'
services:
  mcp-server:
    image: mcp-server:latest
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
      - ./config:/etc/mcp-server
    environment:
      - DATABASE_URL=sqlite:///data/tasks.sqlite
      - CONFIG_FILE=/etc/mcp-server/production.toml
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped
```

### Health Monitoring

#### Health Check Endpoint

```bash
curl http://localhost:3000/health
```

Response:

```json
{
    "status": "healthy",
    "timestamp": "2025-01-29T10:30:00Z",
    "version": "0.1.0",
    "database": {
        "status": "connected",
        "response_time_ms": 2.3
    },
    "server": {
        "uptime_seconds": 86400,
        "active_connections": 15
    }
}
```

#### Metrics Endpoint

```bash
curl http://localhost:3000/metrics
```

Prometheus-formatted metrics:

```
# HELP mcp_requests_total Total number of MCP requests
# TYPE mcp_requests_total counter
mcp_requests_total{method="create_task"} 1250
mcp_requests_total{method="get_task_by_id"} 3400

# HELP mcp_request_duration_seconds MCP request duration
# TYPE mcp_request_duration_seconds histogram
mcp_request_duration_seconds_bucket{method="create_task",le="0.001"} 800
mcp_request_duration_seconds_bucket{method="create_task",le="0.01"} 1200
```

### Backup and Recovery

#### Database Backup

```bash
# Online backup while server is running
sqlite3 /path/to/tasks.sqlite ".backup /backups/tasks_$(date +%Y%m%d_%H%M%S).sqlite"

# Automated backup script
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
sqlite3 "$DATABASE_URL" ".backup /backups/tasks_$DATE.sqlite"
find /backups -name "tasks_*.sqlite" -mtime +7 -delete
```

#### Configuration Backup

```bash
# Backup configuration
tar -czf config_backup_$(date +%Y%m%d).tar.gz config/

# Backup logs
tar -czf logs_backup_$(date +%Y%m%d).tar.gz /var/log/mcp-server/
```

## Troubleshooting

### Common Issues

#### Server Won't Start

**Check configuration**:
```bash
./mcp-server --config config/production.toml 2>&1 | grep -i error
```

**Validate configuration file**:
```bash
# Server validates on startup and reports issues
./mcp-server --config invalid.toml
# Error: Invalid configuration: server.listen_addr is required
```

**Check port availability**:
```bash  
netstat -tlnp | grep :3000
# If port is in use, change listen_addr or stop conflicting service
```

#### Database Connection Issues

**Check database file permissions**:
```bash
ls -la /path/to/database.sqlite
# Should be readable/writable by server user
```

**Test database connection**:
```bash
sqlite3 /path/to/database.sqlite ".tables"
# Should show: tasks (and possibly migration tables)
```

**Check disk space**:
```bash
df -h /path/to/database/
# Ensure sufficient space for database operations
```

#### Performance Issues

**Check resource usage**:
```bash
# Memory usage
ps aux | grep mcp-server
# File descriptor usage  
lsof -p $(pgrep mcp-server) | wc -l
```

**Database performance**:
```bash
# Enable query logging
RUST_LOG=database=debug ./mcp-server

# Check slow queries in logs
grep "slow_query" /var/log/mcp-server/app.log
```

**Connection pool exhaustion**:
```bash
# Check active connections
curl http://localhost:3000/metrics | grep database_connections
```

### Debug Mode

Enable comprehensive debugging:

```bash
RUST_LOG=trace ./mcp-server --config debug.toml
```

Debug configuration (`debug.toml`):

```toml
[logging]
level = "trace"
format = "pretty"
enable_colors = true

[database]
url = "sqlite:///debug.sqlite"
# Lower connection pool for easier debugging
max_connections = 2

[server]
listen_addr = "127.0.0.1:3001"
# Lower timeout for faster failure detection
timeout_seconds = 5
```

### Log Analysis

#### Common Log Patterns

**Successful request**:
```json
{
    "timestamp": "2025-01-29T10:30:00Z",
    "level": "INFO",
    "target": "mcp_server",
    "message": "Request completed",
    "method": "create_task",
    "request_id": "req-123",
    "duration_ms": 2.3,
    "status": "success"
}
```

**Error handling**:
```json
{
    "timestamp": "2025-01-29T10:30:05Z", 
    "level": "ERROR", 
    "target": "mcp_server",
    "message": "Request failed",
    "method": "get_task_by_id",
    "request_id": "req-124",
    "error": "TaskNotFound",
    "task_id": 999
}
```

#### Log Analysis Commands

```bash
# Count requests by method
jq -r '.method' /var/log/mcp-server/app.log | sort | uniq -c

# Average response times
jq -r '.duration_ms' /var/log/mcp-server/app.log | awk '{sum+=$1; count++} END {print sum/count}'

# Error rate
grep '"level":"ERROR"' /var/log/mcp-server/app.log | wc -l
```

## Development

### Building from Source

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Build with specific features
cargo build --features "metrics,tracing"
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# With coverage
cargo tarpaulin --out html
```

### Configuration Validation

The server validates configuration on startup:

```rust
use mcp_server::Config;

let config = Config::from_file("config/test.toml")?;
config.validate()?; // Returns validation errors if any
```

Custom validation rules:

- `listen_addr` must be valid socket address
- `database.url` must be valid SQLite URL
- `logging.level` must be valid log level
- `timeout_seconds` must be positive
- File paths must be accessible

### Adding Configuration Options

1. **Update Config Struct**:
```rust
// In src/config.rs
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub new_option: String,
    // ... existing fields
}
```

2. **Add Validation**:
```rust
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.server.new_option.is_empty() {
            return Err(anyhow!("server.new_option cannot be empty"));
        }
        // ... existing validation
    }
}
```

3. **Update Default Configuration**:
```toml
# In config/default.toml
[server]
new_option = "default_value"
```

4. **Add CLI Option**:
```rust
// In src/main.rs
#[derive(Parser)]
struct Cli {
    #[arg(long, env = "NEW_OPTION")]
    new_option: Option<String>,
    // ... existing fields
}
```

## Architecture

### Application Structure

```
mcp-server/
├── src/
│   ├── main.rs         # CLI and application entry point
│   ├── config.rs       # Configuration management
│   ├── setup.rs        # Application initialization
│   └── telemetry.rs    # Logging and tracing setup
├── config/
│   ├── default.toml    # Default configuration
│   └── production.toml # Production configuration template
└── tests/
    └── integration.rs  # End-to-end tests
```

### Initialization Flow

1. **Parse CLI Arguments**: Command-line options and environment variables
2. **Load Configuration**: File-based config with CLI overrides
3. **Initialize Telemetry**: Logging, tracing, and metrics setup
4. **Validate Configuration**: Comprehensive validation with clear error messages
5. **Setup Database**: Create repository with connection pooling
6. **Create Server**: Initialize MCP server with request handlers
7. **Start Server**: Bind to address and begin accepting connections
8. **Graceful Shutdown**: Handle signals and cleanup resources

### Dependencies

- **clap**: Command-line argument parsing with derive macros
- **anyhow**: Error handling with context
- **tokio**: Async runtime with signal handling
- **serde**: Configuration deserialization
- **tracing**: Structured logging and instrumentation
- **dotenvy**: Environment variable loading from .env files
- **mcp-protocol**: MCP server implementation
- **database**: SQLite repository implementation
- **task-core**: Core business logic and types

## Performance Tuning

### Database Optimization

```toml
[database]
# Increase connection pool for high load
max_connections = 50

# Reduce connection timeout for faster failures
connection_timeout_seconds = 5

# Enable WAL mode for better concurrent performance
enable_wal_mode = true
```

### Server Optimization

```toml
[server]
# Increase connection limit for high load
max_connections = 2000

# Reduce timeout for faster client feedback
timeout_seconds = 15
```

### System Optimization

```bash
# Increase file descriptor limits
ulimit -n 65536

# Tune TCP settings for high connection count
echo 'net.core.somaxconn = 65536' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_max_syn_backlog = 65536' >> /etc/sysctl.conf
sysctl -p
```

## Security

### Running as Non-Root User

```bash
# Create dedicated user
sudo useradd -r -s /bin/false mcp-server

# Set file permissions
sudo chown -R mcp-server:mcp-server /opt/mcp-server
sudo chmod 755 /opt/mcp-server/mcp-server
```

### File System Security

```bash
# Restrict configuration file access
chmod 600 /etc/mcp-server/*.toml

# Create secure data directory
sudo mkdir -p /var/lib/mcp-server
sudo chown mcp-server:mcp-server /var/lib/mcp-server
sudo chmod 700 /var/lib/mcp-server
```

### Network Security

- Bind to localhost (`127.0.0.1`) for local-only access
- Use reverse proxy (nginx/Apache) for public exposure
- Implement TLS termination at proxy level
- Configure firewall rules for port access

## Version

Current version: `0.1.0`

Available via:
```bash
./mcp-server --version
```

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.