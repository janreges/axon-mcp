#!/bin/bash
# JSON Test Case Runner for MCP STDIO Transport
# Usage: ./run_json_test.sh [test_case_path]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

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

# Check if jq is available
if ! command -v jq >/dev/null 2>&1; then
    print_error "jq is required but not installed. Please install jq first."
    echo "  On macOS: brew install jq"
    echo "  On Ubuntu: sudo apt-get install jq"
    exit 1
fi

# Check if server binary exists
if [ ! -f "./target/release/mcp-server" ]; then
    print_info "Building MCP server..."
    if cargo build -p mcp-server --release --quiet; then
        print_status "Server built successfully"
    else
        print_error "Failed to build server"
        exit 1
    fi
fi

JSON_FILE="stdio_test_cases.json"

# Function to run a single test case
run_test_case() {
    local test_path="$1"
    local description="$2"
    
    print_info "Running test: $description"
    
    local test_json
    test_json=$(jq -c "$test_path" "$JSON_FILE")
    
    if [ "$test_json" = "null" ]; then
        print_error "Test case not found: $test_path"
        return 1
    fi
    
    print_info "Request: $(echo "$test_json" | jq -c .)"
    
    local response
    if command -v timeout >/dev/null 2>&1; then
        response=$(echo "$test_json" | timeout 5 ./target/release/mcp-server --transport stdio 2>/dev/null || true)
    elif command -v gtimeout >/dev/null 2>&1; then
        response=$(echo "$test_json" | gtimeout 5 ./target/release/mcp-server --transport stdio 2>/dev/null || true)
    else
        # Fallback for macOS without timeout
        response=$(echo "$test_json" | (./target/release/mcp-server --transport stdio &
            server_pid=$!
            sleep 5
            kill $server_pid 2>/dev/null || true
            wait $server_pid 2>/dev/null || true) 2>/dev/null || true)
    fi
    
    if [ -n "$response" ]; then
        print_status "Response received"
        echo "$response" | jq . 2>/dev/null || echo "$response"
    else
        print_warning "No response received (may be expected for notifications)"
    fi
    
    echo
}

# Function to run handshake test
run_handshake_test() {
    print_info "Running MCP handshake test..."
    
    local temp_script=$(mktemp)
    cat > "$temp_script" << 'EOF'
#!/bin/bash
(
    jq -c '.mcp_protocol_test_cases.handshake.initialize_request' stdio_test_cases.json
    sleep 0.1
    jq -c '.mcp_protocol_test_cases.handshake.initialized_notification' stdio_test_cases.json
    sleep 1
) | ./target/release/mcp-server --transport stdio
EOF
    chmod +x "$temp_script"
    
    local output
    if command -v timeout >/dev/null 2>&1; then
        output=$(timeout 10 "$temp_script" 2>&1)
    elif command -v gtimeout >/dev/null 2>&1; then
        output=$(gtimeout 10 "$temp_script" 2>&1)
    else
        # Fallback for macOS without timeout
        output=$("$temp_script" &
            script_pid=$!
            sleep 10
            kill $script_pid 2>/dev/null || true
            wait $script_pid 2>/dev/null || true) 2>&1
    fi
    
    rm -f "$temp_script"
    
    if echo "$output" | grep -q '"protocolVersion":"2024-11-05"'; then
        print_status "Handshake test passed"
        echo "Initialize response:"
        echo "$output" | jq . 2>/dev/null || echo "$output"
    else
        print_error "Handshake test failed"
        echo "Output: $output"
    fi
    
    echo
}

# Function to run workflow test
run_workflow_test() {
    print_info "Running complete workflow test..."
    
    local temp_script=$(mktemp)
    cat > "$temp_script" << 'EOF'
#!/bin/bash
(
    # Initialize
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[0].message' stdio_test_cases.json
    sleep 0.1
    
    # Initialized notification
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[1].message' stdio_test_cases.json
    sleep 0.1
    
    # Create task
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[2].message' stdio_test_cases.json
    sleep 0.1
    
    # Verify creation
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[3].message' stdio_test_cases.json
    sleep 0.1
    
    # Start work
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[4].message' stdio_test_cases.json
    sleep 0.1
    
    # Update details
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[5].message' stdio_test_cases.json
    sleep 0.1
    
    # Reassign
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[6].message' stdio_test_cases.json
    sleep 0.1
    
    # Complete task
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[7].message' stdio_test_cases.json
    sleep 0.1
    
    # Archive
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[8].message' stdio_test_cases.json
    sleep 0.1
    
    # List final state
    jq -c '.mcp_protocol_test_cases.workflow_test.steps[9].message' stdio_test_cases.json
    
    sleep 2
) | ./target/release/mcp-server --transport stdio
EOF
    chmod +x "$temp_script"
    
    local workflow_output
    if command -v timeout >/dev/null 2>&1; then
        workflow_output=$(timeout 15 "$temp_script" 2>&1)
    elif command -v gtimeout >/dev/null 2>&1; then
        workflow_output=$(gtimeout 15 "$temp_script" 2>&1)
    else
        # Fallback for macOS without timeout
        workflow_output=$("$temp_script" &
            script_pid=$!
            sleep 15
            kill $script_pid 2>/dev/null || true
            wait $script_pid 2>/dev/null || true) 2>&1
    fi
    
    rm -f "$temp_script"
    
    local response_count
    response_count=$(echo "$workflow_output" | grep -c '"result":' || true)
    
    if [ "$response_count" -ge 7 ]; then
        print_status "Workflow test completed successfully ($response_count responses)"
        
        # Show key workflow steps
        if echo "$workflow_output" | grep -q '"code":"WF-001"'; then
            print_status "✓ Task created"
        fi
        
        if echo "$workflow_output" | grep -q '"state":"InProgress"'; then
            print_status "✓ State changed to InProgress"
        fi
        
        if echo "$workflow_output" | grep -q '"state":"Done"'; then
            print_status "✓ Task completed"
        fi
        
        if echo "$workflow_output" | grep -q '"state":"Archived"'; then
            print_status "✓ Task archived"
        fi
        
    else
        print_error "Workflow test failed ($response_count responses)"
        echo "Output sample:"
        echo "$workflow_output" | head -10
    fi
    
    echo
}

# Function to show available test cases
show_test_cases() {
    echo "Available test cases in $JSON_FILE:"
    echo
    
    echo "Handshake tests:"
    echo "  handshake - Complete initialize/initialized flow"
    echo
    
    echo "Individual tool tests:"
    jq -r '.mcp_protocol_test_cases.tool_calls | keys[]' "$JSON_FILE" | sed 's/^/  /'
    echo
    
    echo "Error tests:"
    jq -r '.mcp_protocol_test_cases.error_test_cases | keys[]' "$JSON_FILE" | sed 's/^/  /'
    echo
    
    echo "Complex tests:"
    echo "  workflow - Complete task lifecycle"
    echo
    
    echo "Usage examples:"
    echo "  $0 handshake"
    echo "  $0 workflow"
    echo "  $0 tool health_check"
    echo "  $0 tool create_task"
    echo "  $0 error invalid_json"
    echo "  $0 path '.mcp_protocol_test_cases.tool_calls.list_tasks'"
}

# Main execution
main() {
    case "${1:-help}" in
        "help"|"-h"|"--help")
            echo "JSON Test Case Runner for MCP STDIO Transport"
            echo "Usage: $0 [command] [args...]"
            echo
            echo "Commands:"
            echo "  help           - Show this help"
            echo "  list           - List available test cases" 
            echo "  handshake      - Run handshake test"
            echo "  workflow       - Run complete workflow test"
            echo "  tool [name]    - Run specific tool test"
            echo "  error [name]   - Run specific error test"
            echo "  path [jq_path] - Run test at JSON path"
            echo
            show_test_cases
            ;;
        "list")
            show_test_cases
            ;;
        "handshake")
            run_handshake_test
            ;;
        "workflow")
            run_workflow_test
            ;;
        "tool")
            if [ -z "$2" ]; then
                print_error "Tool name required"
                echo "Available tools:"
                jq -r '.mcp_protocol_test_cases.tool_calls | keys[]' "$JSON_FILE" | sed 's/^/  /'
                exit 1
            fi
            run_test_case ".mcp_protocol_test_cases.tool_calls.$2" "Tool test: $2"
            ;;
        "error")
            if [ -z "$2" ]; then
                print_error "Error test name required"
                echo "Available error tests:"
                jq -r '.mcp_protocol_test_cases.error_test_cases | keys[]' "$JSON_FILE" | sed 's/^/  /'
                exit 1
            fi
            run_test_case ".mcp_protocol_test_cases.error_test_cases.$2" "Error test: $2"
            ;;
        "path")
            if [ -z "$2" ]; then
                print_error "JSON path required"
                echo "Example: $0 path '.mcp_protocol_test_cases.tool_calls.health_check'"
                exit 1
            fi
            run_test_case "$2" "Custom path: $2"
            ;;
        *)
            print_error "Unknown command: $1"
            echo "Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Check if JSON file exists
if [ ! -f "$JSON_FILE" ]; then
    print_error "Test cases file not found: $JSON_FILE"
    echo "Make sure $JSON_FILE exists in the current directory"
    exit 1
fi

main "$@"