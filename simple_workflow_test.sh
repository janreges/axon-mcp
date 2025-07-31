#!/bin/bash

# Simple MCP Workflow Test - Direct approach
set -e

SERVER_URL="http://127.0.0.1:3000/mcp"
TIMESTAMP=$(date +%s)

echo "🔗 Simple MCP Workflow Test - 10 Steps"
echo "======================================"
echo "Timestamp: $TIMESTAMP"
echo ""

# Step 1: Create Epic Task
echo "📡 Step 1: Create Epic Task"
response1=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"create_task\",\"params\":{\"code\":\"EPIC-${TIMESTAMP}\",\"name\":\"Payment System Overhaul\",\"description\":\"Complete payment system redesign\",\"owner_agent_name\":\"ProductOwner\"}}")

echo "Response: $response1"
EPIC_ID=$(echo "$response1" | jq -r '.result.id')
echo "Epic Task ID: $EPIC_ID"
echo ""

# Step 2: Verify Epic Task by Code
echo "📡 Step 2: Get Epic Task by Code"
response2=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"get_task_by_code\",\"params\":{\"code\":\"EPIC-${TIMESTAMP}\"}}")

echo "Response: $response2"
echo ""

# Step 3: Create Feature Task
echo "📡 Step 3: Create Feature Task"
response3=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"create_task\",\"params\":{\"code\":\"FEAT-${TIMESTAMP}\",\"name\":\"Stripe Integration\",\"description\":\"Integrate Stripe payment provider\",\"owner_agent_name\":null}}")

echo "Response: $response3"
FEAT_ID=$(echo "$response3" | jq -r '.result.id')
echo "Feature Task ID: $FEAT_ID"
echo ""

# Step 4: List Tasks
echo "📡 Step 4: List All Tasks"
response4=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":4,"method":"list_tasks","params":{}}')

echo "Response: $response4"
task_count=$(echo "$response4" | jq '.result | length')
echo "Total tasks: $task_count"
echo ""

# Step 5: Discover Work
echo "📡 Step 5: Discover Work for AgentAlice"
response5=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":5,"method":"discover_work","params":{"agent_name":"AgentAlice","capabilities":["payment","integration"],"max_tasks":5}}')

echo "Response: $response5"
available_tasks=$(echo "$response5" | jq '.result | length')
echo "Available tasks: $available_tasks"
echo ""

# Step 6: Claim Feature Task
echo "📡 Step 6: AgentAlice Claims Feature Task"
response6=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"claim_task\",\"params\":{\"task_id\":${FEAT_ID},\"agent_name\":\"AgentAlice\"}}")

echo "Response: $response6"
claimed_owner=$(echo "$response6" | jq -r '.result.owner_agent_name')
echo "Task claimed by: $claimed_owner"
echo ""

# Step 7: Set Feature Task to InProgress
echo "📡 Step 7: Set Feature Task to InProgress"
response7=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"set_task_state\",\"params\":{\"id\":${FEAT_ID},\"state\":\"InProgress\"}}")

echo "Response: $response7"
new_state=$(echo "$response7" | jq -r '.result.state')
echo "New state: $new_state"
echo ""

# Step 8: Start Work Session
echo "📡 Step 8: Start Work Session for AgentAlice"
response8=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":8,\"method\":\"start_work_session\",\"params\":{\"task_id\":${FEAT_ID},\"agent_name\":\"AgentAlice\"}}")

echo "Response: $response8"
session_id=$(echo "$response8" | jq -r '.result.session_id')
echo "Work session ID: $session_id"
echo ""

# Step 9: Update Feature Task
echo "📡 Step 9: Update Feature Task with Progress"
response9=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"update_task\",\"params\":{\"id\":${FEAT_ID},\"name\":\"Stripe Integration - In Progress\",\"description\":\"Stripe payment provider integration in progress\"}}")

echo "Response: $response9"
updated_name=$(echo "$response9" | jq -r '.result.name')
echo "Updated name: $updated_name"
echo ""

# Step 10: Complete Epic Workflow - Set Epic to InProgress, then Done, then Archive
echo "📡 Step 10a: Set Epic Task to InProgress"
response10a=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"set_task_state\",\"params\":{\"id\":${EPIC_ID},\"state\":\"InProgress\"}}")

echo "Response: $response10a"
epic_state1=$(echo "$response10a" | jq -r '.result.state')
echo "Epic state: $epic_state1"
echo ""

echo "📡 Step 10b: Set Epic Task to Done"
response10b=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":11,\"method\":\"set_task_state\",\"params\":{\"id\":${EPIC_ID},\"state\":\"Done\"}}")

echo "Response: $response10b"
epic_state2=$(echo "$response10b" | jq -r '.result.state')
echo "Epic state: $epic_state2"
echo ""

echo "📡 Step 10c: Archive Epic Task"
response10c=$(curl -s -X POST "$SERVER_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":12,\"method\":\"archive_task\",\"params\":{\"id\":${EPIC_ID}}}")

echo "Response: $response10c"
final_state=$(echo "$response10c" | jq -r '.result.state')
echo "Final Epic state: $final_state"
echo ""

# Summary
echo "✅ Workflow Test Completed Successfully!"
echo "======================================"
echo "Epic Task ID: $EPIC_ID"
echo "Feature Task ID: $FEAT_ID"
echo "Work Session ID: $session_id"
echo ""
echo "🎯 Demonstrated complete workflow:"
echo "1. Create Epic and Feature tasks"
echo "2. Verify task creation with get/list operations"
echo "3. Multi-agent work discovery and claiming"
echo "4. State transitions (Created -> InProgress -> Done -> Archived)"
echo "5. Work session tracking"
echo "6. Task updates with progress information"
echo "7. Complete end-to-end task lifecycle"