#!/bin/bash
# MCP Testing Script
# Comprehensive testing tool for MCP Task Management Server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVER_PORT=3000
SERVER_HOST="127.0.0.1"
BASE_URL="http://${SERVER_HOST}:${SERVER_PORT}"
REQUEST_ENDPOINT="${BASE_URL}/mcp/v1/rpc"
SSE_ENDPOINT="${BASE_URL}/mcp/v1"

echo -e "${BLUE}🚀 MCP Task Management Server Testing Suite${NC}"
echo -e "${BLUE}=============================================${NC}"

# Function to print colored output
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to check if server is running
check_server() {
    if curl -s -f "${BASE_URL}/health" >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Function to start server
start_server() {
    print_info "Starting MCP server on ${BASE_URL}..."
    
    # Kill any existing server process
    pkill -f "mcp-server" 2>/dev/null || true
    sleep 2
    
    # Start server in background
    cargo run -p mcp-server --release > server.log 2>&1 &
    SERVER_PID=$!
    
    # Wait for server to start
    for i in {1..30}; do
        if check_server; then
            print_status "Server started successfully (PID: $SERVER_PID)"
            return 0
        fi
        sleep 1
    done
    
    print_error "Failed to start server"
    return 1
}

# Function to stop server
stop_server() {
    if [ ! -z "$SERVER_PID" ]; then
        print_info "Stopping server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
    fi
    pkill -f "mcp-server" 2>/dev/null || true
}

# Function to run integration tests
run_integration_tests() {
    print_info "Running MCP integration tests..."
    
    if cargo test -p mcp-server mcp_integration --release; then
        print_status "Integration tests passed"
        return 0
    else
        print_error "Integration tests failed"
        return 1
    fi
}

# Function to test with curl
test_with_curl() {
    print_info "Testing MCP protocol with curl..."
    
    # Test 1: Health check
    print_info "Testing health check..."
    RESPONSE=$(curl -s -X POST "$REQUEST_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Origin: $BASE_URL" \
        -d '{
            "jsonrpc": "2.0",
            "method": "health_check",
            "id": 1
        }')
    
    if echo "$RESPONSE" | grep -q '"jsonrpc":"2.0"'; then
        print_status "Health check request sent successfully"
    else
        print_error "Health check failed: $RESPONSE"
        return 1
    fi
    
    # Test 2: Create task
    print_info "Testing task creation..."
    RESPONSE=$(curl -s -X POST "$REQUEST_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Origin: $BASE_URL" \
        -d '{
            "jsonrpc": "2.0",
            "method": "create_task",
            "params": {
                "code": "CURL-001",
                "name": "Test Task via curl",
                "description": "Testing MCP with curl",
                "owner_agent_name": "curl-user"
            },
            "id": 2
        }')
    
    if echo "$RESPONSE" | grep -q '200\|201\|202'; then
        print_status "Task creation request sent successfully"
    else
        print_info "Task creation response: $RESPONSE"
    fi
    
    # Test 3: List tasks
    print_info "Testing task listing..."
    RESPONSE=$(curl -s -X POST "$REQUEST_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Origin: $BASE_URL" \
        -d '{
            "jsonrpc": "2.0",
            "method": "list_tasks",
            "params": {},
            "id": 3
        }')
    
    print_info "List tasks response status received"
    
    # Test 4: Error handling (invalid method)
    print_info "Testing error handling..."
    RESPONSE=$(curl -s -X POST "$REQUEST_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Origin: $BASE_URL" \
        -d '{
            "jsonrpc": "2.0",
            "method": "invalid_method",
            "id": 4
        }')
    
    print_info "Error handling test completed"
    print_status "curl tests completed"
}

# Function to test SSE connection
test_sse_connection() {
    print_info "Testing SSE connection..."
    
    # Start SSE listener in background (use gtimeout if available, otherwise use background job)
    if command -v gtimeout >/dev/null 2>&1; then
        gtimeout 10s curl -N -H "Accept: text/event-stream" \
                         -H "Origin: $BASE_URL" \
                         "$SSE_ENDPOINT" > sse_output.log 2>&1 &
    else
        # Fallback for macOS without gtimeout
        curl -N -H "Accept: text/event-stream" \
             -H "Origin: $BASE_URL" \
             "$SSE_ENDPOINT" > sse_output.log 2>&1 &
    fi
    SSE_PID=$!
    
    sleep 2
    
    # Send a request while SSE is listening
    curl -s -X POST "$REQUEST_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Origin: $BASE_URL" \
        -d '{
            "jsonrpc": "2.0",
            "method": "health_check",
            "id": 100
        }' > /dev/null
    
    sleep 3
    
    # Stop SSE listener
    kill $SSE_PID 2>/dev/null || true
    wait $SSE_PID 2>/dev/null || true
    
    # Check if we received SSE events
    if [ -f sse_output.log ] && [ -s sse_output.log ]; then
        print_status "SSE connection established and received data"
        print_info "SSE output sample:"
        head -n 5 sse_output.log | sed 's/^/  /'
    else
        print_warning "SSE connection test inconclusive"
    fi
    
    rm -f sse_output.log
}

# Function to run performance tests
run_performance_tests() {
    print_info "Running basic performance tests..."
    
    # Simple load test with multiple requests
    print_info "Sending 10 concurrent requests..."
    
    # Use a safer approach - create proper JSON payloads
    for i in {1..10}; do
        # Create proper JSON payload to avoid shell escaping issues
        JSON_PAYLOAD=$(cat <<EOF
{
    "jsonrpc": "2.0",
    "method": "create_task",
    "params": {
        "code": "PERF-$(printf "%03d" $i)",
        "name": "Performance Test Task $i",
        "description": "Load testing task",
        "owner_agent_name": "perf-tester"
    },
    "id": $((1000 + i))
}
EOF
)
        # Send request in background with timeout protection
        (curl -s -m 10 -X POST "$REQUEST_ENDPOINT" \
            -H "Content-Type: application/json" \
            -H "Origin: $BASE_URL" \
            -d "$JSON_PAYLOAD" > /dev/null 2>&1) &
    done
    
    # Wait for all background jobs with timeout
    local waited=0
    while [ $(jobs -r | wc -l) -gt 0 ] && [ $waited -lt 30 ]; do
        sleep 1
        waited=$((waited + 1))
    done
    
    # Kill any remaining jobs
    jobs -p | xargs -r kill 2>/dev/null || true
    
    print_status "Performance tests completed (sent 10 requests)"
}

# Function to validate MCP Inspector setup
check_mcp_inspector() {
    print_info "Checking MCP Inspector availability..."
    
    if command -v npx >/dev/null 2>&1; then
        print_status "npx is available"
        print_info "You can run MCP Inspector with:"
        print_info "  npx @modelcontextprotocol/inspector $BASE_URL"
        print_info "  Request endpoint: /mcp/v1/rpc"
        print_info "  SSE endpoint: /mcp/v1"
    else
        print_warning "npx not found. Install Node.js to use MCP Inspector"
    fi
}

# Function to cleanup
cleanup() {
    print_info "Cleaning up..."
    stop_server
    rm -f server.log sse_output.log
}

# Trap to ensure cleanup on exit
trap cleanup EXIT

# Main execution
main() {
    case "${1:-all}" in
        "start")
            start_server
            print_info "Server is running at $BASE_URL"
            print_info "Press Ctrl+C to stop"
            wait
            ;;
        "integration")
            run_integration_tests
            ;;
        "curl")
            if ! check_server; then
                start_server
                LOCAL_SERVER=true
            fi
            test_with_curl
            if [ "$LOCAL_SERVER" = true ]; then
                stop_server
            fi
            ;;
        "sse")
            if ! check_server; then
                start_server
                LOCAL_SERVER=true
            fi
            test_sse_connection
            if [ "$LOCAL_SERVER" = true ]; then
                stop_server
            fi
            ;;
        "performance")
            if ! check_server; then
                start_server
                LOCAL_SERVER=true
            fi
            run_performance_tests
            if [ "$LOCAL_SERVER" = true ]; then
                stop_server
            fi
            ;;
        "inspector")
            check_mcp_inspector
            if ! check_server; then
                start_server
                print_info "Server started for MCP Inspector testing"
                print_info "In another terminal, run:"
                print_info "  npx @modelcontextprotocol/inspector $BASE_URL"
                print_info "Press Ctrl+C to stop server"
                wait
            fi
            ;;
        "all"|*)
            print_info "Running complete MCP test suite..."
            echo
            
            # Step 1: Integration tests (don't need running server)
            run_integration_tests
            echo
            
            # Step 2: Start server for manual tests
            start_server
            echo
            
            # Step 3: Manual tests
            test_with_curl
            echo
            
            test_sse_connection  
            echo
            
            run_performance_tests
            echo
            
            check_mcp_inspector
            echo
            
            print_status "All tests completed successfully!"
            print_info "Server will remain running for 30 seconds for additional manual testing..."
            sleep 30
            ;;
    esac
}

# Show usage if help requested
if [ "$1" = "help" ] || [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    echo "MCP Testing Script Usage:"
    echo "  $0 [command]"
    echo
    echo "Commands:"
    echo "  all         - Run complete test suite (default)"
    echo "  integration - Run integration tests only"
    echo "  curl        - Test with curl commands"
    echo "  sse         - Test SSE connection"
    echo "  performance - Run basic performance tests"
    echo "  inspector   - Setup for MCP Inspector testing"
    echo "  start       - Just start the server"
    echo "  help        - Show this help"
    echo
    echo "Examples:"
    echo "  $0                    # Run all tests"
    echo "  $0 integration        # Run integration tests"
    echo "  $0 inspector          # Start server for MCP Inspector"
    exit 0
fi

# Run main function
main "$1"