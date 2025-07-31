#!/bin/bash

# MCP Workflow Chain Test - 10 navazujících kroků (FIXED VERSION)
# Test komplexního workflow s write->read->validate pattern

set -e

SERVER_URL="http://127.0.0.1:3000/mcp"
OUTPUT_FILE="workflow_chain_results.json"

# Generate unique suffix for this test run
TIMESTAMP=$(date +%s)
EPIC_CODE="EPIC-${TIMESTAMP}"
FEATURE_CODE="FEAT-${TIMESTAMP}"

echo "🔗 MCP Workflow Chain Test - 10 Navazujících Kroků (FIXED)"
echo "========================================================="
echo "Server: $SERVER_URL"
echo "Output: $OUTPUT_FILE"
echo "Epic Code: $EPIC_CODE"
echo "Feature Code: $FEATURE_CODE"
echo ""

# Initialize results array
echo "[]" > "$OUTPUT_FILE"

# Variables to store task IDs
EPIC_TASK_ID=""
FEATURE_TASK_ID=""

# Function to execute MCP call and store result
execute_mcp_call() {
    local step_num=$1
    local step_name="$2"
    local method="$3"
    local params="$4"
    local expected_check="$5"
    
    echo "📡 Krok ${step_num}: ${step_name}"
    echo "   Method: ${method}"
    
    # Create JSON-RPC request using printf for proper variable substitution
    local request=$(printf '{"jsonrpc":"2.0","id":%d,"method":"%s","params":%s}' \
        "$step_num" "$method" "$params")
    
    # Execute request
    local response=$(curl -s -X POST "$SERVER_URL" \
        -H "Content-Type: application/json" \
        -d "$request")
    
    echo "   Response: $response"
    
    # Store result in file
    local temp_file=$(mktemp)
    jq --argjson step "$step_num" --arg name "$step_name" --arg method "$method" --argjson response "$response" \
        '. += [{"step": $step, "name": $name, "method": $method, "response": $response}]' \
        "$OUTPUT_FILE" > "$temp_file" && mv "$temp_file" "$OUTPUT_FILE"
    
    # Basic success check
    if echo "$response" | jq -e '.error' > /dev/null; then
        echo "   ❌ ERROR: $(echo "$response" | jq -r '.error.message')"
        return 1
    else
        echo "   ✅ SUCCESS"
        
        # Extract task ID if this is a create operation
        if [[ "$method" == "create_task" ]]; then
            local task_id=$(echo "$response" | jq -r '.result.id')
            local task_code=$(echo "$response" | jq -r '.result.code')
            echo "   📋 Created Task ID: $task_id, Code: $task_code"
            
            if [[ "$task_code" == "$EPIC_CODE" ]]; then
                EPIC_TASK_ID="$task_id"
            elif [[ "$task_code" == "$FEATURE_CODE" ]]; then
                FEATURE_TASK_ID="$task_id"
            fi
        fi
        
        # Additional validation if specified
        if [ -n "$expected_check" ]; then
            echo "   🔍 Validating: $expected_check"
            if echo "$response" | jq -e "$expected_check" > /dev/null; then
                echo "   ✅ Validation passed"
            else
                echo "   ❌ Validation failed"
                return 1
            fi
        fi
    fi
    
    echo ""
    return 0
}

# KROK 1: Vytvoření Epic tasku
execute_mcp_call 1 \
    "Vytvoření Epic tasku - $EPIC_CODE" \
    "create_task" \
    "{\"code\":\"$EPIC_CODE\",\"name\":\"Payment System Overhaul\",\"description\":\"Complete redesign of payment processing system\",\"owner_agent_name\":\"ProductOwner\"}" \
    ".result.code == \"$EPIC_CODE\""

# KROK 2: Ověření Epic tasku přes get_task_by_code
execute_mcp_call 2 \
    "Ověření Epic tasku - čtení podle kódu" \
    "get_task_by_code" \
    "{\"code\":\"$EPIC_CODE\"}" \
    '.result.owner_agent_name == "ProductOwner" and .result.state == "Created"'

# KROK 3: Vytvoření Feature tasku pro Epic
execute_mcp_call 3 \
    "Vytvoření Feature tasku - $FEATURE_CODE" \
    "create_task" \
    "{\"code\":\"$FEATURE_CODE\",\"name\":\"Integrate New Payment Provider\",\"description\":\"Integrate Stripe as new payment provider\",\"owner_agent_name\":null}" \
    ".result.code == \"$FEATURE_CODE\""

# KROK 4: Listing všech tasků s filtrováním - ověření obou tasků
execute_mcp_call 4 \
    "Listing tasků - ověření vytvoření" \
    "list_tasks" \
    '{}' \
    '.result | length >= 2'

# KROK 5: Agent Alice objevuje dostupnou práci
execute_mcp_call 5 \
    "Discover work pro AgentAlice" \
    "discover_work" \
    '{"agent_name":"AgentAlice","capabilities":["payment","integration"],"max_tasks":5}' \
    '.result | length > 0'

# KROK 6: Agent Alice si zabere Feature task (použije extrahované ID)
echo "🔧 Using Feature Task ID: $FEATURE_TASK_ID"
execute_mcp_call 6 \
    "Agent Alice zabírá $FEATURE_CODE task" \
    "claim_task" \
    "{\"task_id\":$FEATURE_TASK_ID,\"agent_name\":\"AgentAlice\"}" \
    '.result.owner_agent_name == "AgentAlice"'

# KROK 7: Změna stavu na InProgress + ověření
execute_mcp_call 7 \
    "Přechod $FEATURE_CODE na InProgress" \
    "set_task_state" \
    "{\"id\":$FEATURE_TASK_ID,\"state\":\"InProgress\"}" \
    '.result.state == "InProgress"'

# KROK 8: Start pracovní session pro Alice
execute_mcp_call 8 \
    "Start work session pro Alice" \
    "start_work_session" \
    "{\"task_id\":$FEATURE_TASK_ID,\"agent_name\":\"AgentAlice\"}" \
    '.result.session_id'

# KROK 9: Update task s progress informacemi
execute_mcp_call 9 \
    "Update tasku s progress info" \
    "update_task" \
    "{\"id\":$FEATURE_TASK_ID,\"name\":\"Integrate Stripe Payment Provider\",\"description\":\"Integration in progress - API keys configured\"}" \
    '.result.name | contains("Stripe")'

# KROK 10: Kompletní workflow završení - archivace Epic tasku
# Nejdříve musíme převést Epic na Done stav
echo "🔧 Using Epic Task ID: $EPIC_TASK_ID"
execute_mcp_call 10a \
    "Převod Epic na InProgress" \
    "set_task_state" \
    "{\"id\":$EPIC_TASK_ID,\"state\":\"InProgress\"}" \
    '.result.state == "InProgress"'

execute_mcp_call 10b \
    "Převod Epic na Done" \
    "set_task_state" \
    "{\"id\":$EPIC_TASK_ID,\"state\":\"Done\"}" \
    '.result.state == "Done"'

execute_mcp_call 10 \
    "Finální archivace Epic tasku" \
    "archive_task" \
    "{\"id\":$EPIC_TASK_ID}" \
    '.result.state == "Archived"'

echo "✅ Workflow chain test dokončen!"
echo "📊 Výsledky uloženy do: $OUTPUT_FILE"
echo ""

# Sumarizace výsledků
echo "📈 Souhrn workflow testu:"
echo "========================"

# Count successful vs failed steps
successful=$(jq '[.[] | select(.response.error == null)] | length' "$OUTPUT_FILE")
failed=$(jq '[.[] | select(.response.error != null)] | length' "$OUTPUT_FILE")
total=$(jq 'length' "$OUTPUT_FILE")

echo "Celkem kroků: $total"
echo "Úspěšných:   $successful"
echo "Chybných:     $failed"

if [ "$failed" -eq 0 ]; then
    echo "🎉 Všechny kroky workflow proběhly úspěšně!"
else
    echo "⚠️ Některé kroky selhaly, zkontrolujte detaily v $OUTPUT_FILE"
fi

echo ""
echo "🔗 Workflow simuloval:"
echo "1. Vytváření hierarchických tasků (Epic -> Feature)"
echo "2. Ověřování dat přes get/list operace"
echo "3. Multi-agent coordination (discover_work, claim_task)"
echo "4. State management (Created -> InProgress -> Done -> Archived)"
echo "5. Work sessions tracking"
echo "6. Komplexní data flow mezi write a read operacemi"
echo "7. Dynamic task ID extraction and usage"
echo "8. Complete end-to-end workflow validation"

echo ""
echo "📋 Task IDs použité v testu:"
echo "Epic Task ID:    $EPIC_TASK_ID"
echo "Feature Task ID: $FEATURE_TASK_ID"

echo ""
echo "🎯 Workflow chain test dokončen!"