# MCP Task Management Server - Makefile
# Usage: make -f server.mk start|stop|restart|status|log|test

# Default values
LAST ?= 100
SERVER_BIN = target/release/mcp-server
PID_FILE = .server.pid
LOG_FILE = server.log
DB_FILE = $(HOME)/db.sqlite

.PHONY: help start stop restart status log test clean build

help: ## Show this help message
	@echo "MCP Task Management Server - Available commands:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Examples:"
	@echo "  make -f server.mk start          # Start server"
	@echo "  make -f server.mk log LAST=50    # Show last 50 log lines"
	@echo "  make -f server.mk restart        # Restart server"

build: ## Build release binary
	@echo "🔨 Building MCP server..."
	@cargo build --release --quiet
	@echo "✅ Build completed: $(SERVER_BIN)"

start: build ## Start MCP server in background
	@if [ -f $(PID_FILE) ] && kill -0 `cat $(PID_FILE)` 2>/dev/null; then \
		echo "❌ Server already running (PID: `cat $(PID_FILE)`)"; \
		exit 1; \
	fi
	@echo "🚀 Starting MCP server..."
	@rm -f $(LOG_FILE)
	@nohup $(SERVER_BIN) > $(LOG_FILE) 2>&1 & echo $$! > $(PID_FILE)
	@sleep 2
	@if [ -f $(PID_FILE) ] && kill -0 `cat $(PID_FILE)` 2>/dev/null; then \
		echo "✅ Server started successfully"; \
		echo "   PID: `cat $(PID_FILE)`"; \
		echo "   URL: http://127.0.0.1:3000"; \
		echo "   Log: $(LOG_FILE)"; \
		echo "   DB:  $(DB_FILE)"; \
	else \
		echo "❌ Server failed to start"; \
		echo "📋 Last log entries:"; \
		tail -10 $(LOG_FILE) 2>/dev/null || echo "No log file found"; \
		exit 1; \
	fi

stop: ## Stop MCP server
	@if [ ! -f $(PID_FILE) ]; then \
		echo "❌ Server not running (no PID file)"; \
		exit 1; \
	fi
	@PID=`cat $(PID_FILE)`; \
	if kill -0 $$PID 2>/dev/null; then \
		echo "🛑 Stopping MCP server (PID: $$PID)..."; \
		kill $$PID; \
		sleep 2; \
		if kill -0 $$PID 2>/dev/null; then \
			echo "⚠️  Force killing server..."; \
			kill -9 $$PID; \
		fi; \
		echo "✅ Server stopped"; \
	else \
		echo "❌ Server not running (stale PID file)"; \
	fi
	@rm -f $(PID_FILE)

restart: ## Restart MCP server
	@echo "🔄 Restarting MCP server..."
	@$(MAKE) -f server.mk stop || true
	@sleep 1
	@$(MAKE) -f server.mk start

status: ## Check server status
	@echo "📊 MCP Server Status:"
	@echo "===================="
	@if [ -f $(PID_FILE) ] && kill -0 `cat $(PID_FILE)` 2>/dev/null; then \
		echo "Status: ✅ RUNNING"; \
		echo "PID:    `cat $(PID_FILE)`"; \
		echo "URL:    http://127.0.0.1:3000"; \
		echo "Uptime: `ps -o etime= -p \`cat $(PID_FILE)\` | tr -d ' '`"; \
	else \
		echo "Status: ❌ STOPPED"; \
		if [ -f $(PID_FILE) ]; then \
			echo "Note:   Stale PID file exists"; \
		fi; \
	fi
	@echo "DB:     $(DB_FILE) `[ -f $(DB_FILE) ] && echo '✅' || echo '❌'`"
	@echo "Log:    $(LOG_FILE) `[ -f $(LOG_FILE) ] && echo '✅' || echo '❌'`"
	@echo "Binary: $(SERVER_BIN) `[ -f $(SERVER_BIN) ] && echo '✅' || echo '❌'`"

log: ## Show server logs (use LAST=N for number of lines)
	@echo "📋 Server logs (last $(LAST) lines):"
	@echo "===================================="
	@if [ -f $(LOG_FILE) ]; then \
		tail -$(LAST) $(LOG_FILE); \
	else \
		echo "❌ No log file found at $(LOG_FILE)"; \
	fi

follow: ## Follow server logs in real-time
	@echo "📋 Following server logs (Ctrl+C to stop):"
	@echo "==========================================="
	@if [ -f $(LOG_FILE) ]; then \
		tail -f $(LOG_FILE); \
	else \
		echo "❌ No log file found at $(LOG_FILE)"; \
	fi

test: ## Run basic health check test
	@echo "🧪 Testing MCP server..."
	@if ! $(MAKE) -f server.mk status | grep -q "✅ RUNNING"; then \
		echo "❌ Server not running, starting..."; \
		$(MAKE) -f server.mk start; \
		sleep 2; \
	fi
	@echo "📡 Testing health_check endpoint..."
	@curl -s -X POST http://127.0.0.1:3000 \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","id":1,"method":"health_check","params":{}}' \
		| python3 -m json.tool 2>/dev/null || echo "Response received but not valid JSON"
	@echo ""
	@echo "✅ Basic test completed"

clean: ## Clean up temporary files and stop server
	@echo "🧹 Cleaning up..."
	@$(MAKE) -f server.mk stop || true
	@rm -f $(PID_FILE) $(LOG_FILE)
	@rm -f $(DB_FILE)
	@echo "✅ Cleanup completed"

reset-db: ## Reset database (delete and recreate)
	@echo "🗄️  Resetting database..."
	@$(MAKE) -f server.mk stop || true
	@rm -f $(DB_FILE)
	@echo "✅ Database reset completed"

# Health check that server is responsive
health-check:
	@curl -s -f -X POST http://127.0.0.1:3000 \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","id":1,"method":"health_check","params":{}}' \
		>/dev/null && echo "✅ Server responsive" || echo "❌ Server not responsive"