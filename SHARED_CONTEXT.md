# Shared Context Files - CRITICAL COORDINATION SYSTEM

## MANDATORY RULES - ALL AGENTS MUST FOLLOW

### 1. APPEND-ONLY PROTOCOL
**NEVER overwrite these files. ALWAYS append using `>>`**

```bash
# CORRECT - Append to file
echo "[CORE-COMPLETE] 2025-01-29 14:32 rust-architect: Core crate ready" >> STATUS.md

# WRONG - This overwrites!
echo "Core complete" > STATUS.md  # FORBIDDEN!
```

### 2. STANDARDIZED STATUS CODES

These EXACT codes must be used for grep operations. NO VARIATIONS ALLOWED:

#### Crate Status Codes
- `[CORE-COMPLETE]` - Core crate finished
- `[DATABASE-COMPLETE]` - Database crate finished  
- `[PROTOCOL-COMPLETE]` - MCP-protocol crate finished
- `[MOCKS-COMPLETE]` - Mocks crate finished
- `[SERVER-COMPLETE]` - MCP-server crate finished

#### Phase Status Codes
- `[PHASE-1-COMPLETE]` - Core development done
- `[PHASE-2-COMPLETE]` - Parallel development done
- `[PHASE-3-COMPLETE]` - Documentation done
- `[PHASE-4-COMPLETE]` - Finalization done

#### Blocking Issue Codes
- `[BLOCKED-INTERFACE]` - Waiting for interface definition
- `[BLOCKED-DEPENDENCY]` - Waiting for another crate
- `[BLOCKED-TEST]` - Tests failing
- `[BLOCKED-BUILD]` - Build errors

#### Interface Ready Codes
- `[INTERFACE-TASK-REPOSITORY]` - TaskRepository trait ready
- `[INTERFACE-PROTOCOL-HANDLER]` - ProtocolHandler trait ready
- `[INTERFACE-TASK-MODEL]` - Task struct ready
- `[INTERFACE-ERROR-TYPES]` - Error types ready

### 3. GREP USAGE EXAMPLES

```bash
# Check if core is complete before starting
if grep -q "\[CORE-COMPLETE\]" STATUS.md; then
    echo "[DATABASE-START] $(date +%Y-%m-%d\ %H:%M) database-engineer: Starting database crate" >> STATUS.md
fi

# Check for blocking issues
if grep "\[BLOCKED-" STATUS.md | grep -v RESOLVED; then
    echo "Found blocking issues, investigating..."
fi

# Check specific interface readiness
if grep -q "\[INTERFACE-TASK-REPOSITORY\]" INTERFACES.md; then
    echo "TaskRepository trait available, implementing..."
fi
```

### 4. FILE STRUCTURE

#### STATUS.md Format
```
[STATUS-CODE] TIMESTAMP AGENT: Message
[CORE-COMPLETE] 2025-01-29 14:32 rust-architect: Core crate ready with all traits
[DATABASE-START] 2025-01-29 14:35 database-engineer: Beginning SQLite implementation
```

#### INTERFACES.md Format
```
[INTERFACE-CODE] TIMESTAMP AGENT: Description
--- BEGIN DEFINITION ---
<actual interface code>
--- END DEFINITION ---
```

#### DECISIONS.md Format
```
[DECISION-CODE] TIMESTAMP AGENT: Decision summary
RATIONALE: Why this decision was made
ALTERNATIVES: What else was considered
```

### 5. MANDATORY WRITING TRIGGERS

Agents MUST write to shared files when:

1. **Starting work**: `echo "[CRATE-START] $(date +%Y-%m-%d\ %H:%M) agent: Starting X crate" >> STATUS.md`
2. **Completing work**: `echo "[CRATE-COMPLETE] $(date +%Y-%m-%d\ %H:%M) agent: X crate ready" >> STATUS.md`
3. **Blocked**: `echo "[BLOCKED-TYPE] $(date +%Y-%m-%d\ %H:%M) agent: Blocked because..." >> STATUS.md`
4. **Unblocked**: `echo "[BLOCKED-TYPE-RESOLVED] $(date +%Y-%m-%d\ %H:%M) agent: Unblocked" >> STATUS.md`
5. **Interface ready**: `echo "[INTERFACE-NAME] $(date +%Y-%m-%d\ %H:%M) agent: Interface ready" >> INTERFACES.md`

### 6. READING BEFORE WRITING

Always check current status before adding:
```bash
# Check last status for your crate
grep "\[DATABASE-" STATUS.md | tail -5

# Check if already marked complete
if ! grep -q "\[DATABASE-COMPLETE\]" STATUS.md; then
    echo "[DATABASE-COMPLETE] $(date +%Y-%m-%d\ %H:%M) database-engineer: Database crate ready" >> STATUS.md
fi
```

## ENFORCEMENT

Control agent will verify agents are using these codes correctly. Agents not following the protocol will be asked to correct their approach.