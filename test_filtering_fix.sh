#!/bin/bash

# Test filtering fix for Test 38 issue
SERVER_URL="http://127.0.0.1:3000/mcp"

echo "🔧 Testing Filtering Fix - Test 38 Specific"
echo "=========================================="
echo "Server: $SERVER_URL"
echo ""

# Create test data with specific ownership
echo "📝 Creating tasks for filtering test..."
curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"create_task","params":{"code":"AGENTA-001","name":"Task for AgentA","description":"This task belongs to AgentA","owner_agent_name":"AgentA"}}' > /dev/null

curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"create_task","params":{"code":"AGENTB-001","name":"Task for AgentB","description":"This task belongs to AgentB","owner_agent_name":"AgentB"}}' > /dev/null

curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"create_task","params":{"code":"AGENTA-002","name":"Another task for AgentA","description":"This also belongs to AgentA","owner_agent_name":"AgentA"}}' > /dev/null

echo "✅ Created 3 test tasks: 2 for AgentA, 1 for AgentB"
echo ""

# Test 1: List all tasks
echo "📋 Test 1: List all tasks"
ALL_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"list_tasks","params":{}}')

ALL_COUNT=$(echo "$ALL_RESPONSE" | jq '.result | length')
echo "All tasks returned: $ALL_COUNT"
echo ""

# Test 2: Filter by AgentA (THIS WAS THE BUG!)
echo "🔍 Test 2: Filter by AgentA (THE CRITICAL TEST)"
AGENTA_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":5,"method":"list_tasks","params":{"owner":"AgentA"}}')

echo "AgentA filter response: $AGENTA_RESPONSE"
AGENTA_COUNT=$(echo "$AGENTA_RESPONSE" | jq '.result | length')
echo "AgentA tasks returned: $AGENTA_COUNT"

# Verify all returned tasks belong to AgentA
AGENTA_OWNERS=$(echo "$AGENTA_RESPONSE" | jq -r '.result[].owner_agent_name' | sort | uniq)
echo "Owners in AgentA result: $AGENTA_OWNERS"

if [ "$AGENTA_COUNT" -eq 2 ] && [ "$AGENTA_OWNERS" = "AgentA" ]; then
    echo "✅ AgentA filtering WORKS CORRECTLY!"
else
    echo "❌ AgentA filtering still broken - got $AGENTA_COUNT tasks, owners: $AGENTA_OWNERS"
fi
echo ""

# Test 3: Filter by AgentB
echo "🔍 Test 3: Filter by AgentB"
AGENTB_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":6,"method":"list_tasks","params":{"owner":"AgentB"}}')

AGENTB_COUNT=$(echo "$AGENTB_RESPONSE" | jq '.result | length')
echo "AgentB tasks returned: $AGENTB_COUNT"

if [ "$AGENTB_COUNT" -eq 1 ]; then
    echo "✅ AgentB filtering works correctly!"
else
    echo "❌ AgentB filtering broken - got $AGENTB_COUNT tasks, expected 1"
fi
echo ""

# Test 4: Filter by non-existent agent
echo "🔍 Test 4: Filter by non-existent agent"
NONE_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":7,"method":"list_tasks","params":{"owner":"NonExistentAgent"}}')

NONE_COUNT=$(echo "$NONE_RESPONSE" | jq '.result | length')
echo "Non-existent agent tasks returned: $NONE_COUNT"

if [ "$NONE_COUNT" -eq 0 ]; then
    echo "✅ Non-existent agent filtering works correctly!"
else
    echo "❌ Non-existent agent filtering broken - got $NONE_COUNT tasks, expected 0"
fi
echo ""

echo "🎯 Filtering test completed!"
echo ""
echo "SUMMARY:"
echo "- All tasks: $ALL_COUNT"
echo "- AgentA tasks: $AGENTA_COUNT (expected: 2)"  
echo "- AgentB tasks: $AGENTB_COUNT (expected: 1)"
echo "- Non-existent agent: $NONE_COUNT (expected: 0)"

if [ "$AGENTA_COUNT" -eq 2 ] && [ "$AGENTB_COUNT" -eq 1 ] && [ "$NONE_COUNT" -eq 0 ]; then
    echo ""
    echo "🎉 FILTERING FIX SUCCESSFUL! Test 38 bug is resolved!"
else
    echo ""
    echo "⚠️  Filtering fix incomplete - some tests failed"
fi