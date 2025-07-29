# Troubleshooting Guide

Complete troubleshooting guide for the MCP Task Management Server.

## Quick Diagnostics

### Health Check

First, verify the server is running and accessible:

```bash
# Check if server is responding
curl -f http://localhost:3000/health

# Expected response:
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

### Process Check

```bash
# Check if server process is running
ps aux | grep mcp-server

# Check listening ports
netstat -tlnp | grep :3000
# or
ss -tlnp | grep :3000
```

### Log Check

```bash
# System service logs
sudo journalctl -u mcp-server -n 50

# Application logs (if configured)
tail -f /var/log/mcp-server/app.log

# Docker logs
docker logs mcp-server -f
```

## Common Issues

### 1. Server Won't Start

#### Configuration Errors

**Problem**: Server exits with configuration validation error

```
Error: Invalid configuration: server.listen_addr is required
```

**Solution**: Check configuration file syntax and required fields

```bash
# Validate configuration manually
mcp-server --config config/production.toml --help

# Check TOML syntax
cat config/production.toml | python -c "import sys, toml; toml.load(sys.stdin)"
```

**Problem**: Invalid listen address format

```
Error: invalid socket address syntax
```

**Solution**: Ensure proper address format

```toml
[server]
# Correct formats
listen_addr = "127.0.0.1:3000"      # IPv4
listen_addr = "[::1]:3000"          # IPv6
listen_addr = "0.0.0.0:3000"        # All interfaces

# Incorrect formats
listen_addr = "localhost:3000"       # Use IP address
listen_addr = ":3000"               # Missing IP
```

#### Permission Issues

**Problem**: Permission denied errors

```
Error: Permission denied (os error 13)
```

**Solution**: Check file and directory permissions

```bash
# Check binary permissions
ls -la /usr/local/bin/mcp-server
sudo chmod +x /usr/local/bin/mcp-server

# Check configuration file permissions
ls -la /etc/mcp-server/production.toml
sudo chmod 644 /etc/mcp-server/production.toml

# Check data directory permissions
ls -la /var/lib/mcp-server/
sudo chown -R mcp-server:mcp-server /var/lib/mcp-server/
```

#### Port Binding Issues

**Problem**: Address already in use

```
Error: Address already in use (os error 98)
```

**Solution**: Find and resolve port conflicts

```bash
# Find process using port 3000
sudo lsof -i :3000
sudo netstat -tlnp | grep :3000

# Kill conflicting process (if safe)
sudo kill -9 <PID>

# Or change port in configuration
[server]
listen_addr = "127.0.0.1:3001"
```

### 2. Database Issues

#### Database Connection Failures

**Problem**: Database connection errors

```
Error: Database connection failed: unable to open database file
```

**Solution**: Check database file and directory

```bash
# Check if database file exists and is accessible
ls -la /var/lib/mcp-server/tasks.sqlite

# Check directory permissions
ls -la /var/lib/mcp-server/
sudo chown mcp-server:mcp-server /var/lib/mcp-server/

# Check disk space
df -h /var/lib/mcp-server/

# Test database manually
sqlite3 /var/lib/mcp-server/tasks.sqlite ".tables"
```

#### Migration Failures

**Problem**: Database migration errors

```
Error: Migration failed: table tasks already exists
```

**Solutions**:

1. **Reset database (development only)**:
```bash
# Backup first
cp tasks.sqlite tasks.sqlite.backup

# Delete database (will be recreated)
rm tasks.sqlite

# Restart server
systemctl restart mcp-server
```

2. **Manual migration check**:
```bash
sqlite3 tasks.sqlite "SELECT name FROM sqlite_master WHERE type='table';"
```

#### Database Corruption

**Problem**: Database corruption errors

```
Error: database disk image is malformed
```

**Solution**: Recover from corruption

```bash
# Attempt automatic recovery
sqlite3 corrupted.sqlite ".recover" | sqlite3 recovered.sqlite

# Or restore from backup
cp /var/backups/mcp-server/tasks_latest.sqlite tasks.sqlite

# Check integrity
sqlite3 tasks.sqlite "PRAGMA integrity_check;"
```

### 3. Network and Connectivity

#### Connection Timeouts

**Problem**: Client connection timeouts

**Symptoms**:
- HTTP requests timing out
- SSE connections dropping
- Intermittent connectivity

**Solutions**:

1. **Check server timeout settings**:
```toml
[server]
timeout_seconds = 30  # Increase if needed
```

2. **Check network connectivity**:
```bash
# Test from same machine
curl -m 5 http://localhost:3000/health

# Test from remote machine
curl -m 5 http://server-ip:3000/health

# Check firewall rules
sudo ufw status
sudo iptables -L
```

3. **Check resource limits**:
```bash
# Check file descriptor limits
ulimit -n

# Check process limits
cat /proc/$(pgrep mcp-server)/limits
```

#### SSL/TLS Issues

**Problem**: SSL certificate errors when using reverse proxy

**Solution**: Check proxy configuration

```nginx
# Nginx SSL configuration
server {
    listen 443 ssl;
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_ssl_verify off;  # If using self-signed certs
    }
}
```

### 4. Performance Issues

#### High Memory Usage

**Problem**: Server consuming excessive memory

**Diagnosis**:
```bash
# Check memory usage
ps aux | grep mcp-server

# Check detailed memory breakdown
cat /proc/$(pgrep mcp-server)/status | grep -i mem

# Monitor over time
watch -n 5 'ps aux | grep mcp-server'
```

**Solutions**:

1. **Tune database connection pool**:
```toml
[database]
max_connections = 10  # Reduce if memory constrained
idle_timeout_seconds = 300  # Close idle connections faster
```

2. **Enable connection limits**:
```toml
[server]
max_connections = 100  # Limit concurrent connections
```

#### High CPU Usage

**Problem**: Server using high CPU

**Diagnosis**:
```bash
# Check CPU usage
top -p $(pgrep mcp-server)

# Profile with perf (Linux)
sudo perf top -p $(pgrep mcp-server)
```

**Solutions**:

1. **Check for expensive queries**:
```bash
# Enable query logging
RUST_LOG=database=debug systemctl restart mcp-server

# Look for slow queries in logs
grep "slow_query" /var/log/mcp-server/app.log
```

2. **Database optimization**:
```sql
-- Add missing indexes
sqlite3 tasks.sqlite "CREATE INDEX IF NOT EXISTS idx_tasks_state_owner ON tasks(state, owner_agent_name);"

-- Check query plans
sqlite3 tasks.sqlite "EXPLAIN QUERY PLAN SELECT * FROM tasks WHERE state = 'InProgress';"
```

#### Slow Response Times

**Problem**: API requests taking too long

**Diagnosis**:
```bash
# Test response times
time curl http://localhost:3000/mcp/v1/rpc \
  -d '{"jsonrpc":"2.0","id":"1","method":"list_tasks","params":{}}'

# Enable request timing logs
RUST_LOG=mcp_protocol=debug systemctl restart mcp-server
```

**Solutions**:

1. **Database tuning**:
```toml
[database]
# Enable WAL mode for better concurrency
enable_wal_mode = true

# Tune connection pool
max_connections = 20
connection_timeout_seconds = 5
```

2. **Server tuning**:
```toml
[server]
# Reduce timeout for faster failures
timeout_seconds = 15
```

### 5. Protocol and API Issues

#### JSON-RPC Errors

**Problem**: Invalid JSON-RPC requests

**Common errors**:
- `Parse error`: Invalid JSON
- `Invalid Request`: Missing required fields
- `Method not found`: Unsupported method name
- `Invalid params`: Wrong parameter types

**Solutions**:

1. **Validate request format**:
```json
{
    "jsonrpc": "2.0",           // Required
    "id": "unique-id",          // Required
    "method": "create_task",    // Must be valid method
    "params": {                 // Must match method signature
        "code": "TASK-001",
        "name": "Task name",
        "description": "Description",
        "owner_agent_name": "agent"
    }
}
```

2. **Check supported methods**:
```bash
# List supported methods (check API documentation)
curl http://localhost:3000/mcp/v1/rpc \
  -d '{"jsonrpc":"2.0","id":"1","method":"rpc.discover","params":{}}'
```

#### SSE Connection Issues

**Problem**: Server-Sent Events not working

**Symptoms**:
- EventSource connection fails
- No messages received
- Connection drops frequently

**Solutions**:

1. **Check SSE endpoint**:
```javascript
// Test SSE connection
const eventSource = new EventSource('http://localhost:3000/mcp/v1');
eventSource.onopen = () => console.log('Connected');
eventSource.onerror = (e) => console.error('SSE Error:', e);
eventSource.onmessage = (e) => console.log('Message:', e.data);
```

2. **Check proxy configuration** (if using reverse proxy):
```nginx
# Nginx SSE configuration
location /mcp/v1 {
    proxy_pass http://127.0.0.1:3000;
    
    # SSE-specific settings
    proxy_buffering off;
    proxy_cache off;
    proxy_set_header Connection '';
    proxy_http_version 1.1;
    chunked_transfer_encoding off;
}
```

### 6. Docker Issues

#### Container Won't Start

**Problem**: Docker container exits immediately

**Diagnosis**:
```bash
# Check container logs
docker logs mcp-server

# Check exit code
docker ps -a | grep mcp-server
```

**Common solutions**:

1. **Configuration file issues**:
```bash
# Check if config file is mounted correctly
docker exec mcp-server ls -la /config/

# Test configuration
docker run --rm -v $(pwd)/config:/config mcp-server \
  mcp-server --config /config/production.toml --help
```

2. **Permission issues in container**:
```bash
# Check file ownership
docker exec mcp-server ls -la /data/

# Fix ownership
docker exec mcp-server chown -R mcp-server:mcp-server /data/
```

#### Volume Mount Issues

**Problem**: Data not persisting between container restarts

**Solution**: Check volume mounts

```yaml
# docker-compose.yml
services:
  mcp-server:
    volumes:
      - ./data:/data:rw          # Ensure read-write
      - ./config:/config:ro      # Config can be read-only
```

```bash
# Check mount points
docker inspect mcp-server | jq '.[0].Mounts'
```

### 7. Logging and Monitoring

#### Missing Logs

**Problem**: No logs appearing

**Solutions**:

1. **Check log configuration**:
```toml
[logging]
level = "info"
format = "json"
# If directory is set, logs go to files, not stdout
directory = "/var/log/mcp-server"
```

2. **Check log directory permissions**:
```bash
ls -la /var/log/mcp-server/
sudo chown -R mcp-server:mcp-server /var/log/mcp-server/
```

3. **Enable debug logging temporarily**:
```bash
RUST_LOG=debug mcp-server --config config/production.toml
```

#### Log Rotation Issues

**Problem**: Log files growing too large

**Solution**: Set up proper log rotation

```bash
# Create logrotate configuration
sudo tee /etc/logrotate.d/mcp-server << EOF
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
EOF

# Test logrotate
sudo logrotate -d /etc/logrotate.d/mcp-server
```

## Diagnostic Tools

### 1. Built-in Diagnostics

#### Health Check Endpoint

```bash
# Basic health check
curl http://localhost:3000/health

# Detailed health information
curl http://localhost:3000/health?detail=true
```

#### Metrics Endpoint

```bash
# Get Prometheus metrics
curl http://localhost:3000/metrics

# Filter specific metrics
curl http://localhost:3000/metrics | grep mcp_requests
```

### 2. Database Diagnostics

#### Database Integrity Check

```bash
sqlite3 /var/lib/mcp-server/tasks.sqlite << EOF
PRAGMA integrity_check;
PRAGMA foreign_key_check;
.exit
EOF
```

#### Query Performance Analysis

```bash
sqlite3 /var/lib/mcp-server/tasks.sqlite << EOF
.timer on
.headers on
EXPLAIN QUERY PLAN SELECT * FROM tasks WHERE state = 'InProgress';
SELECT COUNT(*) FROM tasks;
.exit
EOF
```

### 3. Network Diagnostics

#### Connection Testing

```bash
# Test TCP connection
telnet localhost 3000

# Test HTTP connection
curl -v http://localhost:3000/health

# Test SSE connection
curl -N -H "Accept: text/event-stream" http://localhost:3000/mcp/v1
```

#### Load Testing

```bash
# Simple load test with curl
for i in {1..100}; do
  curl -s http://localhost:3000/health > /dev/null &
done
wait

# Using ab (Apache Bench)
ab -n 1000 -c 10 http://localhost:3000/health

# Using wrk
wrk -t10 -c100 -d30s http://localhost:3000/health
```

## Recovery Procedures

### 1. Service Recovery

#### Restart Service

```bash
# Systemd service
sudo systemctl restart mcp-server
sudo systemctl status mcp-server

# Docker container
docker restart mcp-server
docker logs mcp-server
```

#### Reset to Known Good State

```bash
# Stop service
sudo systemctl stop mcp-server

# Restore configuration from backup
sudo cp /var/backups/mcp-server-config-latest.tar.gz /tmp/
cd /tmp && sudo tar -xzf mcp-server-config-latest.tar.gz
sudo cp -r etc/mcp-server/* /etc/mcp-server/

# Restore database from backup
sudo cp /var/backups/mcp-server/tasks_latest.sqlite /var/lib/mcp-server/tasks.sqlite
sudo chown mcp-server:mcp-server /var/lib/mcp-server/tasks.sqlite

# Start service
sudo systemctl start mcp-server
```

### 2. Database Recovery

#### From Backup

```bash
# List available backups
ls -la /var/backups/mcp-server/

# Restore specific backup
gunzip -c /var/backups/mcp-server/tasks_20250129_020000.sqlite.gz > /var/lib/mcp-server/tasks.sqlite

# Verify restoration
sqlite3 /var/lib/mcp-server/tasks.sqlite "SELECT COUNT(*) FROM tasks;"
```

#### Emergency Recovery

```bash
# If database is corrupted, try recovery
sqlite3 corrupted.sqlite << EOF
.output recovered.sql
.dump
.exit
EOF

# Create new database from dump
sqlite3 new.sqlite < recovered.sql

# Replace original
mv new.sqlite /var/lib/mcp-server/tasks.sqlite
```

## Prevention

### 1. Monitoring Setup

#### Health Monitoring

```bash
# Create monitoring script
cat > /usr/local/bin/mcp-server-monitor.sh << 'EOF'
#!/bin/bash
if ! curl -f -s http://localhost:3000/health > /dev/null; then
    echo "$(date): MCP Server health check failed" >> /var/log/mcp-server-monitor.log
    systemctl restart mcp-server
fi
EOF

chmod +x /usr/local/bin/mcp-server-monitor.sh

# Add to cron
echo "*/5 * * * * /usr/local/bin/mcp-server-monitor.sh" | crontab -
```

#### Resource Monitoring

```bash
# Monitor disk space
df -h /var/lib/mcp-server/ | awk 'NR==2 {if($5+0 > 80) print "High disk usage: "$5}'

# Monitor memory usage
ps aux | grep mcp-server | awk '{if($4+0 > 10) print "High memory usage: "$4"%"}'
```

### 2. Backup Automation

```bash
# Create backup script
cat > /usr/local/bin/mcp-server-backup.sh << 'EOF'
#!/bin/bash
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/var/backups/mcp-server"

mkdir -p "$BACKUP_DIR"

# Database backup
sqlite3 /var/lib/mcp-server/tasks.sqlite ".backup $BACKUP_DIR/tasks_$DATE.sqlite"
gzip "$BACKUP_DIR/tasks_$DATE.sqlite"

# Configuration backup
tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" /etc/mcp-server/

# Cleanup old backups (keep 30 days)
find "$BACKUP_DIR" -name "*.gz" -mtime +30 -delete

echo "$(date): Backup completed - tasks_$DATE.sqlite.gz" >> /var/log/mcp-server-backup.log
EOF

chmod +x /usr/local/bin/mcp-server-backup.sh

# Schedule daily backups
echo "0 2 * * * /usr/local/bin/mcp-server-backup.sh" | crontab -
```

### 3. Log Rotation

```bash
# Ensure log rotation is properly configured
sudo tee /etc/logrotate.d/mcp-server << EOF
/var/log/mcp-server/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 mcp-server mcp-server
    postrotate
        if systemctl is-active mcp-server > /dev/null; then
            systemctl reload mcp-server
        fi
    endscript
}
EOF
```

## Getting Help

If you can't resolve an issue:

1. **Check this troubleshooting guide** for similar issues
2. **Review the logs** for specific error messages
3. **Search the documentation** for configuration details
4. **Create a GitHub issue** with:
   - Server version (`mcp-server --version`)
   - Operating system and version
   - Configuration file (redacted)
   - Complete error logs
   - Steps to reproduce

### Useful Information to Include

```bash
# System information
uname -a
cat /etc/os-release

# Server version
mcp-server --version

# Configuration (redacted)
cat /etc/mcp-server/production.toml | sed 's/password.*/password=REDACTED/'

# Recent logs
journalctl -u mcp-server -n 100 --no-pager

# Resource usage
ps aux | grep mcp-server
df -h /var/lib/mcp-server/
```

For urgent production issues, include:
- Impact description
- Timeline of events
- Any recent changes
- Current workarounds