#!/bin/bash
# Manual STDIO MCP Testing Script
# Interactive testing tool for MCP Task Management Server STDIO transport

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 MCP STDIO Manual Testing Tool${NC}"
echo -e "${BLUE}=================================${NC}"

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

# Build server if needed
build_server() {
    if [ ! -f "./target/release/mcp-server" ]; then
        print_info "Building MCP server..."
        if cargo build -p mcp-server --release --quiet; then
            print_status "Server built successfully"
        else
            print_error "Failed to build server"
            exit 1
        fi
    else
        print_status "Server binary exists"
    fi
}

# Test basic handshake
test_handshake() {
    print_info "Testing MCP handshake..."
    
    local output
    output=$(
        (
            echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "manual-test", "version": "1.0.0"}}, "id": 1}'
            echo '{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}'
            sleep 1
        ) | timeout 10 ./target/release/mcp-server --transport stdio 2>&1
    )
    
    if echo "$output" | grep -q '"protocolVersion":"2024-11-05"'; then
        print_status "Handshake successful"
        echo "Response excerpt:"
        echo "$output" | head -3 | sed 's/^/  /'
    else
        print_error "Handshake failed"
        echo "Full output:"
        echo "$output"
    fi
    echo
}

# Test single tool call
test_tool() {
    local tool_name="$1"
    local tool_args="$2"
    local description="$3"
    
    print_info "Testing $tool_name: $description"
    
    local output
    output=$(
        (
            echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "tool-test", "version": "1.0.0"}}, "id": 1}'
            echo '{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}'
            echo "{\"jsonrpc\": \"2.0\", \"method\": \"tools/call\", \"params\": {\"name\": \"$tool_name\", \"arguments\": $tool_args}, \"id\": 2}"
            sleep 2
        ) | timeout 10 ./target/release/mcp-server --transport stdio 2>&1
    )
    
    if echo "$output" | grep -q '"result":'; then
        print_status "$tool_name working correctly"
        echo "Result excerpt:"
        echo "$output" | grep '"result":' | head -1 | sed 's/^/  /' | cut -c1-100
        if [ ${#output} -gt 100 ]; then echo "  ..."; fi
    else
        print_error "$tool_name failed"
        echo "Full output:"
        echo "$output" | tail -5
    fi
    echo
}

# Interactive session
run_interactive() {
    print_info "Starting interactive MCP session..."
    print_info "You can now type JSON-RPC messages directly."
    print_info "Start with initialize, then initialized notification, then tool calls."
    print_info "Press Ctrl+C to exit."
    echo
    
    print_info "Example messages:"
    echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "interactive", "version": "1.0.0"}}, "id": 1}'
    echo '{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}'
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "health_check", "arguments": {}}, "id": 2}'
    echo
    print_warning "Starting server - type your messages:"
    
    ./target/release/mcp-server --transport stdio
}

# Full workflow test
test_full_workflow() {
    print_info "Testing complete task management workflow..."
    
    local workflow_script=$(mktemp)
    cat > "$workflow_script" << 'EOF'
#!/bin/bash
(
    echo '{"jsonrpc": "2.0", "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "workflow-test", "version": "1.0.0"}}, "id": 1}'
    sleep 0.2
    echo '{"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}'
    sleep 0.2
    
    # Create a task
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "create_task", "arguments": {"code": "WF-001", "name": "Workflow Test Task", "description": "Testing complete workflow", "owner_agent_name": "workflow-tester"}}, "id": 2}'
    sleep 0.2
    
    # List tasks
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "list_tasks", "arguments": {}}, "id": 3}'
    sleep 0.2
    
    # Get task by ID
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "get_task_by_id", "arguments": {"id": 1}}, "id": 4}'
    sleep 0.2
    
    # Update task
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "update_task", "arguments": {"id": 1, "name": "Updated Workflow Task", "description": "Updated via workflow test"}}, "id": 5}'
    sleep 0.2
    
    # Change state to InProgress
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "set_task_state", "arguments": {"id": 1, "state": "InProgress"}}, "id": 6}'
    sleep 0.2
    
    # Assign to different agent
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "assign_task", "arguments": {"id": 1, "new_owner_agent_name": "workflow-assignee"}}, "id": 7}'
    sleep 0.2
    
    # Mark as done
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "set_task_state", "arguments": {"id": 1, "state": "Done"}}, "id": 8}'
    sleep 0.2
    
    # Archive task
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "archive_task", "arguments": {"id": 1}}, "id": 9}'
    sleep 0.2
    
    # Final list to show archived task
    echo '{"jsonrpc": "2.0", "method": "tools/call", "params": {"name": "list_tasks", "arguments": {}}, "id": 10}'
    
    sleep 3
) | timeout 20 ./target/release/mcp-server --transport stdio
EOF
    chmod +x "$workflow_script"
    
    local workflow_output
    workflow_output=$("$workflow_script" 2>&1)
    
    rm -f "$workflow_script"
    
    # Analyze workflow results
    local success_count
    success_count=$(echo "$workflow_output" | grep -c '"result":' || true)
    
    print_info "Workflow completed with $success_count successful operations"
    
    # Check specific workflow steps
    if echo "$workflow_output" | grep -q '"code":"WF-001"'; then
        print_status "✓ Task created successfully"
    fi
    
    if echo "$workflow_output" | grep -q '"name":"Updated Workflow Task"'; then
        print_status "✓ Task updated successfully"
    fi
    
    if echo "$workflow_output" | grep -q '"state":"InProgress"'; then
        print_status "✓ State changed to InProgress"
    fi
    
    if echo "$workflow_output" | grep -q '"owner_agent_name":"workflow-assignee"'; then
        print_status "✓ Task assigned successfully"
    fi
    
    if echo "$workflow_output" | grep -q '"state":"Archived"'; then
        print_status "✓ Task archived successfully"
    fi
    
    echo
    print_info "Sample workflow output:"
    echo "$workflow_output" | head -10 | sed 's/^/  /'
    echo
}

# Main menu
show_menu() {
    echo "Choose a test option:"
    echo "  1) Test handshake"
    echo "  2) Test individual tools"
    echo "  3) Test complete workflow"
    echo "  4) Interactive session"
    echo "  5) Exit"
    echo
}

# Individual tool tests
test_individual_tools() {
    print_info "Testing individual MCP tools..."
    echo
    
    test_tool "health_check" "{}" "Server health check"
    test_tool "create_task" '{"code": "MANUAL-001", "name": "Manual Test Task", "description": "Testing manual creation", "owner_agent_name": "manual-tester"}' "Create a new task"
    test_tool "list_tasks" "{}" "List all tasks"
    test_tool "get_task_by_id" '{"id": 1}' "Get task by ID"
    test_tool "get_task_by_code" '{"code": "MANUAL-001"}' "Get task by code"
    test_tool "update_task" '{"id": 1, "name": "Updated Manual Task", "description": "Updated description"}' "Update task"
    test_tool "set_task_state" '{"id": 1, "state": "InProgress"}' "Set task state"
    test_tool "assign_task" '{"id": 1, "new_owner_agent_name": "new-manual-owner"}' "Assign task"
    test_tool "archive_task" '{"id": 1}' "Archive task"
}

# Main execution
main() {
    build_server
    echo
    
    case "${1:-menu}" in
        "handshake")
            test_handshake
            ;;
        "tools")
            test_individual_tools
            ;;
        "workflow")
            test_full_workflow
            ;;
        "interactive")
            run_interactive
            ;;
        "menu"|*)
            while true; do
                show_menu
                read -p "Enter your choice (1-5): " choice
                echo
                
                case $choice in
                    1)
                        test_handshake
                        ;;
                    2)
                        test_individual_tools
                        ;;
                    3)
                        test_full_workflow
                        ;;
                    4)
                        run_interactive
                        ;;
                    5)
                        print_info "Goodbye!"
                        exit 0
                        ;;
                    *)
                        print_error "Invalid choice. Please select 1-5."
                        ;;
                esac
                
                if [ "$choice" != "4" ]; then
                    echo
                    read -p "Press Enter to continue..."
                    echo
                fi
            done
            ;;
    esac
}

# Show usage if help requested
if [ "$1" = "help" ] || [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
    echo "Manual STDIO MCP Testing Tool Usage:"
    echo "  $0 [command]"
    echo
    echo "Commands:"
    echo "  menu        - Show interactive menu (default)"
    echo "  handshake   - Test MCP protocol handshake only"
    echo "  tools       - Test all individual tools"
    echo "  workflow    - Test complete task workflow"
    echo "  interactive - Start interactive session"
    echo "  help        - Show this help"
    echo
    echo "Examples:"
    echo "  $0                # Show interactive menu"
    echo "  $0 handshake      # Test handshake only"
    echo "  $0 tools          # Test all tools"
    echo "  $0 interactive    # Interactive JSON-RPC session"
    exit 0
fi

# Run main function
main "$1"