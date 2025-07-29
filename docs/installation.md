# Installation Guide

Complete installation and setup guide for the MCP Task Management Server.

## Prerequisites

### System Requirements

- **Operating System**: Linux, macOS, or Windows
- **Rust**: Version 1.75+ with 2024 edition support
- **Memory**: Minimum 512MB RAM, recommended 2GB+
- **Storage**: 100MB for application, additional space for database
- **Network**: Available port for server binding (default: 3000)

### Installing Rust

If you don't have Rust installed:

```bash
# Install Rust via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add Rust to PATH
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

## Installation Methods

### 1. Build from Source (Recommended)

#### Clone Repository

```bash
git clone <repository-url>
cd task-manager
```

#### Build Release Binary

```bash
# Build optimized release binary
cargo build --release

# Binary location
ls -la target/release/mcp-server
```

#### Verify Installation

```bash
# Check version
./target/release/mcp-server --version

# Test configuration
./target/release/mcp-server --help
```

### 2. Development Installation

For development and testing:

```bash
# Clone repository
git clone <repository-url>
cd task-manager

# Build in development mode
cargo build

# Run directly with cargo
cargo run -- --help

# Run tests
cargo test
```

### 3. Docker Installation

#### Using Pre-built Image

```bash
# Pull image (when available)
docker pull mcp-server:latest

# Run container
docker run -d \
  --name mcp-server \
  -p 3000:3000 \
  -v ./data:/data \
  -e DATABASE_URL="sqlite:///data/tasks.sqlite" \
  mcp-server:latest
```

#### Build Docker Image

```bash
# Build image from source
docker build -t mcp-server .

# Run built image
docker run -d \
  --name mcp-server \
  -p 3000:3000 \
  mcp-server
```

## Initial Setup

### 1. Create Configuration

Create configuration directory and file:

```bash
# Create config directory  
mkdir -p config

# Copy default configuration
cp config/default.toml config/production.toml
```

Edit `config/production.toml`:

```toml
[server]
listen_addr = "127.0.0.1:3000"
timeout_seconds = 30

[database]
# Use absolute path for production
url = "sqlite:///var/lib/mcp-server/tasks.sqlite"
max_connections = 10

[logging]
level = "info"
format = "json"
```

### 2. Database Setup

The server automatically creates and migrates the database:

```bash
# Create data directory
mkdir -p /var/lib/mcp-server

# Set permissions (if running as specific user)
sudo chown mcp-server:mcp-server /var/lib/mcp-server
sudo chmod 755 /var/lib/mcp-server

# First run will create database
./target/release/mcp-server --config config/production.toml
```

### 3. Test Installation

Verify the server is working:

```bash
# Start server in background
./target/release/mcp-server --config config/production.toml &

# Test health check
curl http://localhost:3000/health

# Test MCP endpoint
curl http://localhost:3000/mcp/v1/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":"test","method":"list_tasks","params":{}}'

# Stop background server
pkill mcp-server
```

## Production Deployment

### 1. System User Setup

Create dedicated user for security:

```bash
# Create system user
sudo useradd -r -s /bin/false -d /var/lib/mcp-server mcp-server

# Create directories
sudo mkdir -p /var/lib/mcp-server
sudo mkdir -p /var/log/mcp-server  
sudo mkdir -p /etc/mcp-server

# Set ownership
sudo chown mcp-server:mcp-server /var/lib/mcp-server
sudo chown mcp-server:mcp-server /var/log/mcp-server
```

### 2. Install Binary

```bash
# Copy binary to system location
sudo cp target/release/mcp-server /usr/local/bin/
sudo chmod 755 /usr/local/bin/mcp-server

# Verify system installation
/usr/local/bin/mcp-server --version
```

### 3. Configuration Files

```bash
# Copy configuration to system location
sudo cp config/production.toml /etc/mcp-server/
sudo chown root:mcp-server /etc/mcp-server/production.toml
sudo chmod 640 /etc/mcp-server/production.toml
```

### 4. Systemd Service

Create service file `/etc/systemd/system/mcp-server.service`:

```ini
[Unit]
Description=MCP Task Management Server
Documentation=https://github.com/your-org/mcp-task-server
After=network.target
Wants=network.target

[Service]
Type=simple
User=mcp-server
Group=mcp-server
WorkingDirectory=/var/lib/mcp-server

# Command to run
ExecStart=/usr/local/bin/mcp-server --config /etc/mcp-server/production.toml

# Restart policy
Restart=always
RestartSec=5

# Environment
Environment=RUST_LOG=info

# Security settings
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=/var/lib/mcp-server /var/log/mcp-server

# Limits
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Enable and start service:

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable mcp-server

# Start service
sudo systemctl start mcp-server

# Check status
sudo systemctl status mcp-server

# View logs
sudo journalctl -u mcp-server -f
```

### 5. Firewall Configuration

Configure firewall if needed:

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 3000/tcp

# firewalld (RHEL/CentOS)
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload

# iptables (direct)
sudo iptables -A INPUT -p tcp --dport 3000 -j ACCEPT
```

### 6. Reverse Proxy (Optional)

#### Nginx Configuration

Create `/etc/nginx/sites-available/mcp-server`:

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # SSE support
        proxy_buffering off;
        proxy_cache off;
        proxy_set_header Connection '';
        proxy_http_version 1.1;
        chunked_transfer_encoding off;
    }
}
```

Enable site:

```bash
sudo ln -s /etc/nginx/sites-available/mcp-server /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

#### Apache Configuration

Add to virtual host:

```apache
<VirtualHost *:80>
    ServerName your-domain.com
    
    ProxyPreserveHost On
    ProxyRequests Off
    ProxyPass / http://127.0.0.1:3000/
    ProxyPassReverse / http://127.0.0.1:3000/
    
    # SSE support
    ProxyTimeout 300
    ProxyBadHeader Ignore
</VirtualHost>
```

## Docker Deployment

### 1. Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  mcp-server:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./data:/data
      - ./config:/config
    environment:
      - DATABASE_URL=sqlite:///data/tasks.sqlite
      - CONFIG_FILE=/config/production.toml
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped
    
  # Optional: nginx reverse proxy
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - mcp-server
    restart: unless-stopped
```

Create directories and start:

```bash
# Create necessary directories
mkdir -p data config

# Copy configuration
cp config/default.toml config/production.toml

# Start services
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f mcp-server
```

### 2. Production Docker Setup

For production, use multi-stage builds and security best practices:

```dockerfile
# Multi-stage build
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create user
RUN useradd -r -s /bin/false mcp-server

# Copy binary
COPY --from=builder /app/target/release/mcp-server /usr/local/bin/
RUN chmod 755 /usr/local/bin/mcp-server

# Create directories
RUN mkdir -p /data /config && \
    chown mcp-server:mcp-server /data

USER mcp-server
WORKDIR /data

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

CMD ["mcp-server", "--config", "/config/production.toml"]
```

## Environment-Specific Configurations

### Development Environment

```toml
# config/development.toml
[server]
listen_addr = "127.0.0.1:3001"
timeout_seconds = 5

[database]
url = "sqlite:///dev.sqlite"
max_connections = 2

[logging]
level = "debug"
format = "pretty"
enable_colors = true
```

### Testing Environment

```toml
# config/test.toml
[server]
listen_addr = "127.0.0.1:0"  # Random port
timeout_seconds = 1

[database]  
url = "sqlite::memory:"

[logging]
level = "warn"
format = "compact"
```

### Production Environment

```toml
# config/production.toml
[server]
listen_addr = "0.0.0.0:3000"
timeout_seconds = 30
max_connections = 1000

[database]
url = "sqlite:///var/lib/mcp-server/tasks.sqlite"
max_connections = 50
connection_timeout_seconds = 5

[logging]
level = "info"
format = "json"
directory = "/var/log/mcp-server"
```

## Monitoring Setup

### 1. Health Monitoring

Set up automated health checks:

```bash
#!/bin/bash
# health-check.sh

HEALTH_URL="http://localhost:3000/health"
RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$HEALTH_URL")

if [ "$RESPONSE" -eq 200 ]; then
    echo "Service is healthy"
    exit 0
else
    echo "Service is unhealthy (HTTP $RESPONSE)"
    exit 1
fi
```

Add to cron:

```bash
# Check every 5 minutes
*/5 * * * * /path/to/health-check.sh
```

### 2. Log Monitoring

Set up log rotation:

```bash
# /etc/logrotate.d/mcp-server
/var/log/mcp-server/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 mcp-server mcp-server
    postrotate
        systemctl reload mcp-server
    endscript
}
```

### 3. Metrics Collection

Enable Prometheus metrics:

```toml
[telemetry]
enable_metrics = true
metrics_path = "/metrics"
```

Configure Prometheus scraping:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'mcp-server'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

## Backup and Recovery

### 1. Database Backup

Create backup script:

```bash
#!/bin/bash
# backup.sh

DATABASE_PATH="/var/lib/mcp-server/tasks.sqlite"
BACKUP_DIR="/var/backups/mcp-server"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

# Create backup
sqlite3 "$DATABASE_PATH" ".backup $BACKUP_DIR/tasks_$DATE.sqlite"

# Compress
gzip "$BACKUP_DIR/tasks_$DATE.sqlite"

# Keep only last 30 days
find "$BACKUP_DIR" -name "tasks_*.sqlite.gz" -mtime +30 -delete

echo "Backup completed: tasks_$DATE.sqlite.gz"
```

Schedule backup:

```bash
# Daily backup at 2 AM
0 2 * * * /usr/local/bin/backup.sh
```

### 2. Configuration Backup

```bash
#!/bin/bash
# config-backup.sh

tar -czf "/var/backups/mcp-server-config-$(date +%Y%m%d).tar.gz" \
    /etc/mcp-server/ \
    /etc/systemd/system/mcp-server.service
```

## Troubleshooting Installation

### Common Issues

**Permission denied on binary execution**:
```bash
chmod +x /usr/local/bin/mcp-server
```

**Database permission errors**:
```bash
sudo chown -R mcp-server:mcp-server /var/lib/mcp-server
```

**Port already in use**:
```bash
# Find process using port
sudo lsof -i :3000

# Kill process or change port in config
```

**Service fails to start**:
```bash
# Check service logs
sudo journalctl -u mcp-server -n 50

# Check configuration
/usr/local/bin/mcp-server --config /etc/mcp-server/production.toml
```

### Verification Commands

```bash
# Test binary
mcp-server --version

# Test configuration
mcp-server --config config/production.toml --help

# Test database connection
sqlite3 /var/lib/mcp-server/tasks.sqlite ".tables"

# Test server response
curl -f http://localhost:3000/health
```

## Next Steps

After successful installation:

1. **Read Configuration Guide**: Learn about advanced configuration options
2. **Review API Documentation**: Understand the MCP protocol implementation
3. **Set up Monitoring**: Implement health checks and log monitoring
4. **Plan Backups**: Set up automated database and configuration backups
5. **Security Hardening**: Review security best practices

For more information, see:
- [Configuration Reference](configuration.md)
- [API Documentation](../API.md)
- [Troubleshooting Guide](troubleshooting.md)
- [CONTRIBUTING.md](../CONTRIBUTING.md)