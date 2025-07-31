#!/bin/bash

# Test critical fixes for release_task and end_work_session
SERVER_URL="http://127.0.0.1:3000/mcp"

echo "🔧 Testing Critical Fixes"
echo "========================"
echo "Server: $SERVER_URL"
echo ""

# Test 1: Create unassigned task
echo "📝 Creating unassigned task..."
CREATE_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"create_task","params":{"code":"FIX-TEST-001","name":"Critical Fix Test","description":"Testing critical fixes for release_task and end_work_session","owner_agent_name":null}}')
echo "Create response: $CREATE_RESPONSE"
echo ""

# Test 2: Claim task
echo "📝 Claiming task..."
CLAIM_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"claim_task","params":{"task_id":1,"agent_name":"TestAgent"}}')
echo "Claim response: $CLAIM_RESPONSE"
echo ""

# Test 3: Start work session  
echo "📝 Starting work session..."
START_SESSION_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"start_work_session","params":{"task_id":1,"agent_name":"TestAgent"}}')
echo "Start session response: $START_SESSION_RESPONSE"

# Extract session_id from response
SESSION_ID=$(echo "$START_SESSION_RESPONSE" | jq -r '.result.session_id // 1')
echo "Extracted session_id: $SESSION_ID"
echo ""

# Test 4: End work session (CRITICAL FIX TEST)
echo "🔧 Testing end_work_session fix..."
END_SESSION_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"end_work_session\",\"params\":{\"session_id\":$SESSION_ID,\"notes\":\"Test session completed\",\"productivity_score\":0.9}}")
echo "End session response: $END_SESSION_RESPONSE"

if echo "$END_SESSION_RESPONSE" | jq -e 'has("result")' > /dev/null; then
    echo "✅ end_work_session fix SUCCESSFUL"
else
    echo "❌ end_work_session fix FAILED"
    echo "Error: $(echo "$END_SESSION_RESPONSE" | jq -r '.error.message // "Unknown error"')"
fi
echo ""

# Test 5: Release task (CRITICAL FIX TEST)
echo "🔧 Testing release_task fix..."
RELEASE_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":5,"method":"release_task","params":{"task_id":1,"agent_name":"TestAgent"}}')
echo "Release response: $RELEASE_RESPONSE"

if echo "$RELEASE_RESPONSE" | jq -e 'has("result")' > /dev/null; then
    echo "✅ release_task fix SUCCESSFUL"
else
    echo "❌ release_task fix FAILED"
    echo "Error: $(echo "$RELEASE_RESPONSE" | jq -r '.error.message // "Unknown error"')"
fi
echo ""

# Test 6: Verify task is released (owner_agent_name should be null)
echo "📝 Verifying task release..."
GET_TASK_RESPONSE=$(curl -s -X POST $SERVER_URL \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":6,"method":"get_task_by_id","params":{"id":1}}')
echo "Get task response: $GET_TASK_RESPONSE"

OWNER=$(echo "$GET_TASK_RESPONSE" | jq -r '.result.owner_agent_name // "ERROR"')
OWNER_NULL=$(echo "$GET_TASK_RESPONSE" | jq -r '.result.owner_agent_name')
if [ "$OWNER_NULL" == "null" ]; then
    echo "✅ Task correctly released (owner_agent_name is null)"
else
    echo "❌ Task not properly released (owner_agent_name: $OWNER)"
fi
echo ""

echo "🎯 Critical fixes test completed!"