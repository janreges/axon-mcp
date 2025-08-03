# Axon MCP Task Management System - Production Deployment Guide

## 🚀 Production-Ready Release

**Version:** 1.2.0 - Revolutionary Dynamic Agent Orchestration  
**Release Date:** August 1, 2025  
**Status:** ✅ Production Ready with Pro Model R.I.C.H. Pattern Implementation

## 📋 System Overview

Axon MCP is an intelligent workspace automation system that dynamically generates optimal AI agent teams based on project analysis. It implements cutting-edge **R.I.C.H. prompting patterns** and **ProjectArchetype classification** for maximum efficiency.

### Key Innovations
- **🎯 Smart Team Sizing**: 3 agents for CLI tools vs 8+ for web apps (62.5% resource optimization)  
- **🧠 R.I.C.H. Prompting**: Role-specific, Imperative, Contextual, Handoff-enabled agent coordination
- **⚡ Dynamic Classification**: Automatic project archetype detection with 15 unit-tested patterns
- **🔄 Template-Based**: Extensible handlebars system for future AI tool integration

## 🏗️ Architecture Components

### Production Binaries
- **`mcp-server`** (11.2MB) - Core MCP protocol server with 6 intelligent functions
- **`workspace-orchestrator`** (8.4MB) - Dynamic agent team orchestration with R.I.C.H. patterns

### Core Libraries  
- **`task-core`** (4.6MB) - Domain models, ProjectArchetype classification, validation
- **`database`** (1.4MB) - SQLite repository with optimized queries and migrations
- **`mcp-protocol`** (843KB) - MCP v2 compliance with HTTP/SSE transport
- **`mocks`** (634KB) - Comprehensive testing utilities

## 🚦 Deployment Options

### Option 1: Direct Binary Deployment (Recommended)

```bash
# 1. Copy production binaries
cp target/release/mcp-server /usr/local/bin/
cp target/release/workspace-orchestrator /usr/local/bin/
chmod +x /usr/local/bin/mcp-server
chmod +x /usr/local/bin/workspace-orchestrator

# 2. Create configuration directory
mkdir -p /etc/axon/{templates,schemas}
cp templates/CLAUDE.md.hbs /etc/axon/templates/
cp -r docs/schemas/ /etc/axon/

# 3. Start MCP server
MCP_SERVER_PORT=8080 mcp-server

# 4. Test workspace orchestrator
workspace-orchestrator --poc-test
```

### Option 2: Docker Deployment

```dockerfile
# Dockerfile
FROM alpine:latest
RUN apk add --no-cache ca-certificates sqlite
COPY target/release/mcp-server /usr/local/bin/
COPY target/release/workspace-orchestrator /usr/local/bin/
COPY templates/ /etc/axon/templates/
COPY docs/schemas/ /etc/axon/schemas/
EXPOSE 8080
CMD ["mcp-server"]
```

```bash
# Build and run
docker build -t axon-mcp:1.2.0 .
docker run -d -p 8080:8080 --name axon-mcp axon-mcp:1.2.0
```

### Option 3: Kubernetes Deployment

```yaml
# k8s-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: axon-mcp-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: axon-mcp
  template:
    metadata:
      labels:
        app: axon-mcp
    spec:
      containers:
      - name: mcp-server
        image: axon-mcp:1.2.0
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          value: "sqlite:///data/axon.db"
        volumeMounts:
        - name: data-volume
          mountPath: /data
      volumes:
      - name: data-volume
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: axon-mcp-service
spec:
  selector:
    app: axon-mcp
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer
```

## ⚙️ Configuration

### Environment Variables

```bash
# MCP Server Configuration
export MCP_SERVER_PORT=8080
export MCP_SERVER_HOST=0.0.0.0
export DATABASE_URL=sqlite:///var/lib/axon/db.sqlite
export RUST_LOG=info
export TEMPLATES_PATH=/etc/axon/templates

# Workspace Orchestrator Configuration  
export MCP_SERVER_URL=http://localhost:8080
export TEMPLATE_PATH=/etc/axon/templates/CLAUDE.md.hbs
export OUTPUT_PATH=/tmp/axon-output
```

### Configuration Files

#### `/etc/axon/config.toml`
```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "sqlite:///var/lib/axon/db.sqlite"
max_connections = 10

[templates]
claude_md_path = "/etc/axon/templates/CLAUDE.md.hbs"

[orchestrator]
default_mcp_url = "http://localhost:8080"
max_agents = 12
classification_timeout_ms = 5000

[logging]
level = "info"
format = "json"
```

## 🔧 System Requirements

### Minimum Requirements
- **CPU**: 2 cores, 2.0GHz
- **Memory**: 1GB RAM
- **Storage**: 500MB disk space
- **OS**: Linux (Ubuntu 20.04+), macOS (10.15+), Windows (Server 2019+)

### Recommended Production
- **CPU**: 4 cores, 3.0GHz+
- **Memory**: 4GB RAM
- **Storage**: 2GB SSD
- **Network**: 1Gbps
- **OS**: Linux (Ubuntu 22.04 LTS)

### Dependencies
- **SQLite**: 3.35+ (embedded)
- **OpenSSL**: 1.1.1+ (for HTTPS)
- **libc**: glibc 2.31+ or musl

## 🌐 API Endpoints

### MCP Server Endpoints

```bash
# Health Check
GET http://localhost:8080/health
→ {"status": "healthy", "version": "1.2.0"}

# MCP Protocol (JSON-RPC 2.0)
POST http://localhost:8080/mcp
Content-Type: application/json

# WebSocket (Real-time)
WS ws://localhost:8080/ws
```

### Available MCP Functions

| Function | Purpose | Input | Output |
|----------|---------|-------|--------|
| `get_setup_instructions` | AI tool setup guide | `ai_tool_type` | Setup steps, required functions |
| `get_agentic_workflow_description` | **Core Intelligence** | `prd_content` | Optimized agent team (3-12 agents) |
| `get_main_file_instructions` | Template instructions | `ai_tool_type`, `workflow_context` | File template, variables |
| `create_main_file` | Generate coordination file | `ai_tool_type`, `project_context` | Generated CLAUDE.md with R.I.C.H. prompts |
| `generate_workspace_manifest` | Project manifest | `project_metadata`, `agent_configuration` | JSON/YAML manifest |
| `get_workspace_manifest` | Retrieve manifest | `manifest_path` | Parsed manifest data |

## 🧪 Testing & Validation

### Production Validation
```bash
# 1. System Health Check
curl http://localhost:8080/health

# 2. MCP Function Test
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0", 
    "id": 1,
    "method": "get_setup_instructions",
    "params": {"ai_tool_type": "ClaudeCode"}
  }'

# 3. Orchestrator POC Test
workspace-orchestrator --poc-test

# 4. Load Test (requires hey or ab)
hey -n 1000 -c 10 http://localhost:8080/health
```

### Monitoring Commands
```bash
# Check process status
ps aux | grep mcp-server

# Monitor logs
tail -f /var/log/axon/mcp-server.log

# Database status
sqlite3 /var/lib/axon/db.sqlite ".tables"

# Resource usage
top -p $(pidof mcp-server)
```

## 📊 Performance Metrics

### Benchmarks (Production Hardware)
- **Startup Time**: Fast startup for immediate productivity
- **Memory Usage**: Efficient memory management for server applications
- **Response Time**: Responsive agent generation suitable for interactive use
- **Throughput**: High throughput appropriate for multi-agent coordination
- **Database**: Scalable SQLite performance for task management workloads

### ProjectArchetype Classification Performance
- **Simple CLI**: Quick agent generation for lightweight projects
- **Web Application**: Efficient processing for moderate complexity projects
- **Complex Enterprise**: Comprehensive analysis for large-scale projects
- **Classification Accuracy**: Reliable project archetype detection based on unit tests

## 🔒 Security & Authentication

### Default Security (Production)
```bash
# Enable authentication
export MCP_ENABLE_AUTH=true
export MCP_AUTH_TOKEN=your-secure-token-here

# HTTPS Configuration  
export MCP_TLS_CERT_PATH=/etc/ssl/certs/axon.crt
export MCP_TLS_KEY_PATH=/etc/ssl/private/axon.key
```

### API Security Headers
- **CORS**: Configurable origins
- **Rate Limiting**: 100 requests/minute per IP
- **Request Validation**: JSON Schema validation
- **Error Sanitization**: No internal details exposed

## 🔍 Monitoring & Observability

### Logging Configuration
```bash
# Structured JSON logging (recommended)
export RUST_LOG=info
export LOG_FORMAT=json

# Human-readable logging (development)
export LOG_FORMAT=pretty
```

### Health Monitoring
```bash
# Kubernetes Liveness Probe
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10

# Readiness Probe
readinessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 5
```

### Metrics Collection (Optional)
```bash
# Prometheus metrics endpoint
GET http://localhost:8080/metrics

# Key metrics available:
# - axon_requests_total
# - axon_request_duration_seconds
# - axon_agent_generation_count
# - axon_classification_accuracy
```

## 🚨 Troubleshooting

### Common Issues

#### 1. Database Connection Issues
```bash
# Check database path
ls -la /var/lib/axon/db.sqlite

# Fix permissions
chown axon:axon /var/lib/axon/db.sqlite
chmod 664 /var/lib/axon/db.sqlite

# Reset database
rm /var/lib/axon/db.sqlite
# Restart mcp-server (auto-migrates)
```

#### 2. Template Loading Errors
```bash
# Verify template exists
ls -la /etc/axon/templates/CLAUDE.md.hbs

# Test template syntax
workspace-orchestrator --poc-test

# Fix template permissions
chmod 644 /etc/axon/templates/CLAUDE.md.hbs
```

#### 3. Classification Issues
```bash
# Test classification with debug logging
RUST_LOG=debug workspace-orchestrator --poc-test

# Check for Generic archetype warnings in logs
grep "ARCHETYPE CLASSIFICATION" /var/log/axon/mcp-server.log
```

### Debug Mode
```bash
# Enable verbose logging
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Run in foreground with debug output
mcp-server --debug
```

## 📈 Scaling Considerations

### Horizontal Scaling
- **Load Balancer**: Multiple MCP server instances behind nginx/HAProxy
- **Database**: Consider PostgreSQL for multi-instance deployments
- **Caching**: Redis for agent template caching
- **Queue**: Add background job processing for large PRD analysis

### Vertical Scaling
- **Memory**: Increase for larger PRD processing
- **CPU**: More cores improve concurrent request handling
- **Storage**: SSD for database performance

## 🔄 Backup & Recovery

### Database Backup
```bash
# Daily backup script
#!/bin/bash
sqlite3 /var/lib/axon/db.sqlite ".backup /backup/axon-$(date +%Y%m%d).db"

# Restore from backup
sqlite3 /var/lib/axon/db.sqlite ".restore /backup/axon-20250801.db"
```

### Configuration Backup
```bash
# Backup all configs
tar -czf axon-config-$(date +%Y%m%d).tar.gz \
  /etc/axon/ \
  /var/lib/axon/ \
  /usr/local/bin/mcp-server \
  /usr/local/bin/workspace-orchestrator
```

## 🔧 Maintenance

### Regular Maintenance Tasks
```bash
# 1. Database optimization (monthly)
sqlite3 /var/lib/axon/db.sqlite "VACUUM;"

# 2. Log rotation (daily)
logrotate /etc/logrotate.d/axon

# 3. Update check (weekly)
curl -s https://api.github.com/repos/axon-mcp/releases/latest

# 4. Health verification (daily)
curl -f http://localhost:8080/health || systemctl restart axon-mcp
```

### Upgrade Procedure
```bash
# 1. Stop services
systemctl stop axon-mcp

# 2. Backup current version
cp /usr/local/bin/mcp-server /backup/mcp-server.backup

# 3. Install new version
cp target/release/mcp-server /usr/local/bin/

# 4. Run migrations (automatic on startup)
systemctl start axon-mcp

# 5. Verify upgrade
curl http://localhost:8080/health
```

## 📞 Support & Documentation

### Quick Reference
- **GitHub Repository**: https://github.com/axon-mcp/task-management-system
- **API Documentation**: [docs/schemas/README.md](docs/schemas/README.md)
- **Architecture Guide**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Pro Model Integration**: Implemented R.I.C.H. prompting patterns

### Getting Help
1. **Check logs**: `/var/log/axon/mcp-server.log`
2. **Run diagnostics**: `workspace-orchestrator --poc-test`
3. **Validate schemas**: Use JSON schema validation
4. **Community**: GitHub Issues and Discussions

---

## 🎉 Production Success Metrics

After deployment, you should see:
- ✅ **Efficient resource utilization** for simple projects with appropriate agent sizing
- ✅ **Fast response times** suitable for interactive agent team generation
- ✅ **Reliable classification accuracy** across project types based on testing
- ✅ **Dynamic scaling** from 3 to 12 agents based on complexity
- ✅ **R.I.C.H. pattern compliance** in all generated prompts

**Axon MCP Task Management System v1.2.0 is production-ready for revolutionary AI agent orchestration!** 🚀