#!/bin/bash

# Test Flash Model MCP Scenarios
# 8 konkrétních testovacích scénářů od Flash modelu

set -e

SERVER_URL="http://127.0.0.1:3000/mcp"
OUTPUT_FILE="flash_test_results.json"

echo "🧪 MCP Flash Model Test Scenarios"
echo "================================="
echo "Server: $SERVER_URL"
echo "Output: $OUTPUT_FILE"
echo ""

# Inicializace výsledkového souboru
echo "[]" > $OUTPUT_FILE

# Funkce pro volání MCP
call_mcp() {
    local method=$1
    local params=$2
    local id=$3
    local description=$4
    
    echo "📡 Test #$id: $description"
    echo "   Method: $method"
    
    local response=$(curl -s -X POST $SERVER_URL \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":$id,\"method\":\"$method\",\"params\":$params}")
    
    # Přidání do výsledků  
    local temp_result=$(echo "$response" | jq -r .)
    jq --argjson new_entry "{\"test_id\":$id,\"method\":\"$method\",\"description\":\"$description\",\"request_params\":$params,\"response\":$temp_result}" '. += [$new_entry]' $OUTPUT_FILE > temp.json && mv temp.json $OUTPUT_FILE
    
    echo "   Response: $response"
    echo ""
    
    return 0
}

echo "🎯 Scénář 1: Základní CRUD operace a archivace"
echo "==============================================="

call_mcp "create_task" '{
    "code": "CRUD-001",
    "name": "Testovací úkol pro CRUD",
    "description": "Popis úkolu pro testování základních operací.",
    "owner_agent_name": "AgentX"
}' 1 "Vytvoření úkolu CRUD-001"

call_mcp "get_task_by_id" '{"id": 1}' 2 "Získání úkolu podle ID"

call_mcp "update_task" '{
    "id": 1,
    "description": "Aktualizovaný popis úkolu po revizi.",
    "owner_agent_name": "AgentY"
}' 3 "Aktualizace úkolu"

call_mcp "get_task_by_code" '{"code": "CRUD-001"}' 4 "Získání úkolu podle kódu"

call_mcp "archive_task" '{"id": 1}' 5 "Archivace úkolu"

call_mcp "get_task_by_id" '{"id": 1}' 6 "Kontrola archivovaného úkolu"

echo "🤝 Scénář 2: Koordinace více agentů a pracovní relace"
echo "====================================================="

call_mcp "create_task" '{
    "code": "AGENT-WORK-001",
    "name": "Úkol pro AgentA",
    "description": "Tento úkol vyžaduje schopnosti coding a testing.",
    "owner_agent_name": null
}' 7 "Vytvoření nepřiřazeného úkolu"

call_mcp "discover_work" '{
    "agent_name": "AgentA",
    "capabilities": ["coding", "testing"],
    "max_tasks": 5
}' 8 "Objevení práce pro AgentA"

call_mcp "claim_task" '{
    "task_id": 2,
    "agent_name": "AgentA"
}' 9 "Zabrání úkolu AgentA"

call_mcp "start_work_session" '{
    "task_id": 2,
    "agent_name": "AgentA"
}' 10 "Spuštění pracovní relace"

call_mcp "end_work_session" '{
    "session_id": 2,
    "notes": "Work completed successfully",
    "productivity_score": 0.95
}' 11 "Ukončení pracovní relace"

call_mcp "release_task" '{
    "task_id": 2,
    "agent_name": "AgentA"
}' 12 "Uvolnění úkolu"

call_mcp "get_task_by_id" '{"id": 2}' 13 "Kontrola uvolněného úkolu"

echo "❌ Scénář 3: Operace s neexistujícím úkolem"
echo "=========================================="

call_mcp "get_task_by_id" '{"id": 9999}' 14 "Pokus o získání neexistujícího úkolu"

call_mcp "update_task" '{
    "id": 9999,
    "name": "Neexistující úkol",
    "description": "Tento úkol neexistuje."
}' 15 "Pokus o aktualizaci neexistujícího úkolu"

call_mcp "set_task_state" '{
    "id": 9999,
    "state": "Done"
}' 16 "Pokus o změnu stavu neexistujícího úkolu"

call_mcp "archive_task" '{"id": 9999}' 17 "Pokus o archivaci neexistujícího úkolu"

call_mcp "claim_task" '{
    "task_id": 9999,
    "agent_name": "AgentNotFound"
}' 18 "Pokus o zabrání neexistujícího úkolu"

echo "🔄 Scénář 4: Neplatné přechody stavů úkolu"
echo "========================================="

call_mcp "create_task" '{
    "code": "INVALID-STATE-001",
    "name": "Úkol pro test neplatných stavů",
    "description": "Testuje neplatné přechody stavů.",
    "owner_agent_name": null
}' 19 "Vytvoření úkolu pro test stavů"

call_mcp "set_task_state" '{
    "id": 3,
    "state": "Done"
}' 20 "Pokus o přechod Created → Done"

call_mcp "set_task_state" '{
    "id": 3,
    "state": "Archived"
}' 21 "Pokus o přechod Created → Archived"

call_mcp "set_task_state" '{
    "id": 3,
    "state": "InProgress"
}' 22 "Přechod Created → InProgress"

call_mcp "set_task_state" '{
    "id": 3,
    "state": "Created"
}' 23 "Pokus o přechod InProgress → Created"

echo "🌈 Scénář 5: Kompletní životní cyklus úkolu"
echo "==========================================="

call_mcp "create_task" '{
    "code": "FULL-CYCLE-001",
    "name": "Kompletní životní cyklus úkolu",
    "description": "Testuje plný životní cyklus úkolu s blokováním a přeřazením.",
    "owner_agent_name": "AgentAlpha"
}' 24 "Vytvoření úkolu pro full cycle"

call_mcp "set_task_state" '{"id": 4, "state": "InProgress"}' 25 "Přechod na InProgress"

call_mcp "set_task_state" '{"id": 4, "state": "Blocked"}' 26 "Přechod na Blocked"

call_mcp "set_task_state" '{"id": 4, "state": "InProgress"}' 27 "Návrat na InProgress"

call_mcp "assign_task" '{
    "id": 4,
    "new_owner": "AgentBeta"
}' 28 "Přiřazení jinému agentovi"

call_mcp "set_task_state" '{"id": 4, "state": "Review"}' 29 "Přechod na Review"

call_mcp "set_task_state" '{"id": 4, "state": "Done"}' 30 "Přechod na Done"

call_mcp "archive_task" '{"id": 4}' 31 "Archivace dokončeného úkolu"

call_mcp "get_task_by_id" '{"id": 4}' 32 "Kontrola finálního stavu"

echo "📋 Scénář 6: Filtrování a stránkování seznamu úkolů"
echo "===================================================="

# Vytvoření testovacích úkolů
call_mcp "create_task" '{
    "code": "FILTER-001",
    "name": "Úkol pro AgentA - Created",
    "description": "Popis úkolu 1.",
    "owner_agent_name": "AgentA"
}' 33 "Vytvoření úkolu pro AgentA"

call_mcp "create_task" '{
    "code": "FILTER-002",
    "name": "Úkol pro AgentB - Created",
    "description": "Popis úkolu 2.",
    "owner_agent_name": "AgentB"
}' 34 "Vytvoření úkolu pro AgentB"

call_mcp "create_task" '{
    "code": "FILTER-003", 
    "name": "Úkol pro AgentA - InProgress",
    "description": "Popis úkolu 3.",
    "owner_agent_name": "AgentA"
}' 35 "Vytvoření dalšího úkolu pro AgentA"

call_mcp "set_task_state" '{"id": 7, "state": "InProgress"}' 36 "Změna stavu na InProgress"

# Testování filtrování
call_mcp "list_tasks" '{}' 37 "Seznam všech úkolů"

call_mcp "list_tasks" '{"owner": "AgentA"}' 38 "Úkoly pro AgentA"

call_mcp "list_tasks" '{"state": "InProgress"}' 39 "Úkoly ve stavu InProgress"

call_mcp "list_tasks" '{
    "owner": "AgentA",
    "state": "Created"
}' 40 "Úkoly pro AgentA ve stavu Created"

call_mcp "list_tasks" '{
    "limit": 1,
    "offset": 0
}' 41 "První úkol se stránkováním"

echo "⚡ Scénář 7: Simulace konfliktu při zabírání úkolu"
echo "=================================================="

call_mcp "create_task" '{
    "code": "CLAIM-CONFLICT-001",
    "name": "Úkol pro test konfliktu zabrání",
    "description": "Testuje, co se stane, když dva agenti zabírají stejný úkol.",
    "owner_agent_name": null
}' 42 "Vytvoření úkolu pro konflikt test"

call_mcp "claim_task" '{
    "task_id": 8,
    "agent_name": "AgentA"
}' 43 "AgentA zabírá úkol"

call_mcp "claim_task" '{
    "task_id": 8,
    "agent_name": "AgentB"
}' 44 "AgentB pokus o zabrání stejného úkolu"

call_mcp "release_task" '{
    "task_id": 8,
    "agent_name": "AgentA"
}' 45 "AgentA uvolňuje úkol"

call_mcp "claim_task" '{
    "task_id": 8,
    "agent_name": "AgentB"
}' 46 "AgentB úspěšně zabírá uvolněný úkol"

echo "🔍 Scénář 8: Kontrola stavu a zpracování neplatných parametrů"
echo "============================================================="

call_mcp "health_check" '{}' 47 "Kontrola stavu serveru"

call_mcp "create_task" '{
    "code": "INVALID-PARAMS-001",
    "description": "Tento úkol by měl selhat kvůli chybějícímu názvu."
}' 48 "Vytvoření úkolu s chybějícím názvem"

call_mcp "create_task" '{
    "code": "VALID-FOR-STATE-TEST",
    "name": "Úkol pro test neplatného stavu",
    "description": "Tento úkol bude použit pro pokus o nastavení neplatného stavu.",
    "owner_agent_name": null
}' 49 "Vytvoření platného úkolu"

call_mcp "set_task_state" '{
    "id": 9,
    "state": "InvalidStateString"
}' 50 "Pokus o nastavení neplatného stavu"

echo ""
echo "✅ Všech 8 scénářů dokončeno!"
echo "📊 Výsledky uloženy do: $OUTPUT_FILE"
echo ""
echo "📈 Souhrn testů:"
jq -r '.[] | "\(.test_id). \(.description) - \(if .response.result then "✅ SUCCESS" elif .response.error then "❌ ERROR: \(.response.error.message)" else "❓ UNKNOWN" end)"' $OUTPUT_FILE

echo ""
echo "🎯 Test dokončen - výsledky připraveny pro Flash model!"