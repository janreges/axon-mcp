# Shared Context Files - CRITICAL COORDINATION SYSTEM

## MANDATORY RULES - ALL AGENTS MUST FOLLOW

### 1. USE MAKEFILE FOR ALL OPERATIONS
**NEVER write directly to files. ALWAYS use make commands**

```bash
# CORRECT - Use Makefile targets
make status-complete AGENT=rust-architect CRATE=core

# WRONG - Direct file manipulation is FORBIDDEN!
echo "[CORE-COMPLETE] ..." >> STATUS.md  # NO!
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

### 3. MAKEFILE USAGE EXAMPLES

```bash
# Check if core is complete before starting
if make check-crate CRATE=core | grep -q "complete"; then
    make status-start AGENT=database-engineer CRATE=database
fi

# Check for dependencies
make check-deps  # Will exit with error if core not ready

# Check specific interface readiness
make interface-check INTERFACE=TASK-REPOSITORY

# Report being blocked
make status-blocked AGENT=database-engineer TYPE=INTERFACE MSG="Need TaskRepository trait"

# Mark as unblocked when resolved
make status-unblocked AGENT=database-engineer TYPE=INTERFACE
```

### 4. FILE STRUCTURE

#### STATUS.md Format
```
[STATUS-CODE] TIMESTAMP AGENT: Message
[CORE-COMPLETE] 2025-01-29 14:32:17 rust-architect: Core crate ready with all traits
[DATABASE-START] 2025-01-29 14:35:42 database-engineer: Beginning SQLite implementation
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

Agents MUST use Makefile commands when:

1. **Starting work**: `make status-start AGENT=agent-name CRATE=crate-name`
2. **Completing work**: `make status-complete AGENT=agent-name CRATE=crate-name`
3. **Blocked**: `make status-blocked AGENT=agent-name TYPE=type MSG='reason'`
4. **Unblocked**: `make status-unblocked AGENT=agent-name TYPE=type`
5. **Interface ready**: `make interface-add AGENT=agent-name INTERFACE=name FILE=path/to/file`
6. **Decision made**: `make decision AGENT=agent-name SUMMARY='what' RATIONALE='why' ALTERNATIVES='other options'`
7. **Phase complete**: `make phase-complete AGENT=agent-name PHASE=number`

### 6. READING STATUS

Use Makefile commands to check status:
```bash
# Check overall project status
make check-status

# Check specific crate
make check-crate CRATE=database

# Check if ready for next phase
make check-phase-ready PHASE=2

# Validate all status codes
make validate
```

## ENFORCEMENT

Control agent will verify agents are using these codes correctly. Agents not following the protocol will be asked to correct their approach.