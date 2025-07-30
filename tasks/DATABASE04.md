# DATABASE04: Create Agents Table

## Objective
Implement the agents table with proper constraints, indexes, and triggers to manage agent registration, capabilities, and workload tracking.

## Implementation Details

### 1. Create Migration File
Create `database/migrations/sqlite/006_agents.sql`:

```sql
-- Agents Table
-- Manages AI agent registration, capabilities, and workload
CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL CHECK (
        -- Enforce kebab-case: lowercase letters, numbers, and hyphens only
        name GLOB '[a-z0-9-]*' AND 
        name NOT GLOB '*--*' AND 
        name NOT GLOB '-*' AND 
        name NOT GLOB '*-' AND
        length(name) BETWEEN 3 AND 50
    ),
    description TEXT NOT NULL CHECK (
        length(description) > 0 AND 
        length(description) <= 4000
    ),
    capabilities TEXT NOT NULL, -- JSON array of capability strings
    max_concurrent_tasks INTEGER NOT NULL CHECK (
        max_concurrent_tasks >= 1 AND 
        max_concurrent_tasks <= 100
    ),
    current_load INTEGER DEFAULT 0 CHECK (
        current_load >= 0 AND 
        current_load <= max_concurrent_tasks
    ),
    status TEXT NOT NULL DEFAULT 'idle' CHECK (
        status IN ('idle', 'active', 'blocked', 'unresponsive', 'offline')
    ),
    preferences TEXT, -- JSON object for agent-specific preferences
    last_heartbeat DATETIME DEFAULT CURRENT_TIMESTAMP,
    reputation_score REAL DEFAULT 1.0 CHECK (
        reputation_score >= 0.0 AND 
        reputation_score <= 1.0
    ),
    specializations TEXT, -- JSON array of specialization areas
    registered_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    registered_by TEXT NOT NULL,
    total_tasks_completed INTEGER DEFAULT 0,
    total_tasks_failed INTEGER DEFAULT 0,
    
    -- Computed column for success rate (SQLite 3.31.0+)
    success_rate REAL GENERATED ALWAYS AS (
        CASE 
            WHEN (total_tasks_completed + total_tasks_failed) = 0 THEN 1.0
            ELSE CAST(total_tasks_completed AS REAL) / 
                 CAST(total_tasks_completed + total_tasks_failed AS REAL)
        END
    ) STORED
);

-- Indexes for performance
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_heartbeat ON agents(last_heartbeat);
CREATE INDEX idx_agents_load ON agents(current_load, max_concurrent_tasks);
CREATE INDEX idx_agents_reputation ON agents(reputation_score DESC);

-- JSON indexes for capability searches
CREATE INDEX idx_agents_capabilities ON agents(capabilities);
CREATE INDEX idx_agents_specializations ON agents(specializations);

-- Composite index for work discovery
CREATE INDEX idx_agents_available_work ON agents(status, current_load, reputation_score DESC)
    WHERE status IN ('idle', 'active') AND current_load < max_concurrent_tasks;

-- Trigger to update last_heartbeat on status change
CREATE TRIGGER update_heartbeat_on_status_change
AFTER UPDATE OF status ON agents
FOR EACH ROW
BEGIN
    UPDATE agents 
    SET last_heartbeat = CURRENT_TIMESTAMP 
    WHERE id = NEW.id;
END;

-- Trigger to enforce current_load constraints
CREATE TRIGGER validate_current_load_update
BEFORE UPDATE OF current_load ON agents
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.current_load > NEW.max_concurrent_tasks
        THEN RAISE(ABORT, 'Current load cannot exceed max concurrent tasks')
        WHEN NEW.current_load < 0
        THEN RAISE(ABORT, 'Current load cannot be negative')
    END;
END;

-- Trigger to update reputation score bounds
CREATE TRIGGER validate_reputation_update
BEFORE UPDATE OF reputation_score ON agents
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.reputation_score > 1.0
        THEN RAISE(IGNORE)
        WHEN NEW.reputation_score < 0.0
        THEN RAISE(IGNORE)
    END;
    
    -- Clamp values
    UPDATE agents 
    SET reputation_score = 
        CASE 
            WHEN NEW.reputation_score > 1.0 THEN 1.0
            WHEN NEW.reputation_score < 0.0 THEN 0.0
            ELSE NEW.reputation_score
        END
    WHERE id = NEW.id;
END;

-- View for agent workload summary
CREATE VIEW agent_workload_summary AS
SELECT 
    a.name,
    a.status,
    a.current_load,
    a.max_concurrent_tasks,
    CAST(a.current_load AS REAL) / CAST(a.max_concurrent_tasks AS REAL) * 100 as load_percentage,
    a.reputation_score,
    a.success_rate,
    (strftime('%s', 'now') - strftime('%s', a.last_heartbeat)) as seconds_since_heartbeat,
    COUNT(DISTINCT t.code) as active_task_count,
    json_array_length(a.capabilities) as capability_count
FROM agents a
LEFT JOIN tasks t ON t.owner_agent_name = a.name 
    AND t.state IN ('InProgress', 'Review', 'Blocked')
GROUP BY a.id;

-- View for available agents by capability
CREATE VIEW available_agents_by_capability AS
WITH capability_list AS (
    SELECT 
        a.id,
        a.name,
        a.status,
        a.current_load,
        a.max_concurrent_tasks,
        a.reputation_score,
        json_each.value as capability
    FROM agents a, json_each(a.capabilities)
    WHERE a.status IN ('idle', 'active')
      AND a.current_load < a.max_concurrent_tasks
)
SELECT 
    capability,
    name as agent_name,
    status,
    current_load,
    max_concurrent_tasks - current_load as available_capacity,
    reputation_score
FROM capability_list
ORDER BY capability, reputation_score DESC;

-- View for unresponsive agents (missed 3+ heartbeats)
CREATE VIEW unresponsive_agents AS
SELECT 
    name,
    status,
    last_heartbeat,
    (strftime('%s', 'now') - strftime('%s', last_heartbeat)) / 60 as minutes_since_heartbeat,
    current_load,
    registered_by
FROM agents
WHERE status IN ('idle', 'active', 'blocked')
  AND (strftime('%s', 'now') - strftime('%s', last_heartbeat)) > 180; -- 3 minutes

-- Function to find best agent for capability (using JSON)
-- Note: This would be a query pattern, not a stored function
-- Example query to find best available agent for a capability:
/*
SELECT 
    name,
    reputation_score,
    current_load,
    max_concurrent_tasks - current_load as available_capacity
FROM agents
WHERE status IN ('idle', 'active')
  AND current_load < max_concurrent_tasks
  AND EXISTS (
      SELECT 1 FROM json_each(capabilities)
      WHERE json_each.value = 'rust'  -- capability to search for
  )
ORDER BY 
    reputation_score DESC,
    current_load ASC
LIMIT 1;
*/
```

### 2. Create Rollback Migration
Create `database/migrations/sqlite/006_agents_rollback.sql`:

```sql
-- Rollback agents migration
DROP VIEW IF EXISTS unresponsive_agents;
DROP VIEW IF EXISTS available_agents_by_capability;
DROP VIEW IF EXISTS agent_workload_summary;
DROP TRIGGER IF EXISTS validate_reputation_update;
DROP TRIGGER IF EXISTS validate_current_load_update;
DROP TRIGGER IF EXISTS update_heartbeat_on_status_change;
DROP INDEX IF EXISTS idx_agents_available_work;
DROP INDEX IF EXISTS idx_agents_specializations;
DROP INDEX IF EXISTS idx_agents_capabilities;
DROP INDEX IF EXISTS idx_agents_reputation;
DROP INDEX IF EXISTS idx_agents_load;
DROP INDEX IF EXISTS idx_agents_heartbeat;
DROP INDEX IF EXISTS idx_agents_status;
DROP TABLE IF EXISTS agents;
```

### 3. Create Test Data Script
Create `database/scripts/test_agents.sql`:

```sql
-- Test data for agents
INSERT INTO agents (name, description, capabilities, max_concurrent_tasks, registered_by, specializations) VALUES
(
    'rust-architect',
    'Senior Rust architect responsible for system design and core architecture decisions',
    '["rust", "architecture", "system-design", "async", "traits"]',
    3,
    'system',
    '["distributed-systems", "api-design"]'
),
(
    'frontend-dev',
    'Frontend developer specializing in React and modern web technologies',
    '["javascript", "react", "typescript", "css", "html"]',
    5,
    'system',
    '["react", "performance-optimization"]'
),
(
    'database-engineer',
    'Database specialist focused on SQLite optimization and schema design',
    '["sql", "sqlite", "database-design", "optimization", "migrations"]',
    4,
    'system',
    '["sqlite", "query-optimization"]'
),
(
    'testing-expert',
    'QA engineer specializing in test automation and quality assurance',
    '["testing", "rust", "automation", "mocking", "integration-testing"]',
    6,
    'system',
    '["test-automation", "ci-cd"]'
),
(
    'junior-dev',
    'Junior developer learning the codebase and handling simple tasks',
    '["rust", "documentation", "bug-fixing"]',
    2,
    'system',
    '[]'
);

-- Set some agents as active with load
UPDATE agents SET status = 'active', current_load = 2 WHERE name = 'rust-architect';
UPDATE agents SET status = 'active', current_load = 3 WHERE name = 'frontend-dev';
UPDATE agents SET status = 'blocked', current_load = 1 WHERE name = 'junior-dev';

-- Update task completion stats
UPDATE agents SET 
    total_tasks_completed = 45,
    total_tasks_failed = 5,
    reputation_score = 0.92
WHERE name = 'rust-architect';

UPDATE agents SET 
    total_tasks_completed = 78,
    total_tasks_failed = 3,
    reputation_score = 0.96
WHERE name = 'database-engineer';

-- Simulate an unresponsive agent
UPDATE agents SET 
    last_heartbeat = datetime('now', '-5 minutes')
WHERE name = 'junior-dev';
```

### 4. Create Helper Functions in Database Module
In `database/src/agents.rs`:

```rust
/// Parse capabilities from JSON string
pub fn parse_capabilities(capabilities_json: &str) -> Result<Vec<String>> {
    serde_json::from_str(capabilities_json)
        .map_err(|e| TaskError::Database(format!("Invalid capabilities JSON: {}", e)))
}

/// Convert capabilities to JSON string
pub fn capabilities_to_json(capabilities: &[String]) -> String {
    serde_json::to_string(capabilities).unwrap_or_else(|_| "[]".to_string())
}

/// Check if agent has specific capability
pub fn has_capability(capabilities_json: &str, capability: &str) -> bool {
    parse_capabilities(capabilities_json)
        .map(|caps| caps.iter().any(|c| c == capability))
        .unwrap_or(false)
}

/// Calculate agent suitability score
pub fn calculate_suitability_score(
    capability_match: f64,
    reputation: f64,
    load_percentage: f64,
    is_specialized: bool,
) -> f64 {
    let load_factor = 1.0 - (load_percentage / 100.0).min(1.0);
    let specialization_bonus = if is_specialized { 0.2 } else { 0.0 };
    
    (capability_match * 0.4) + 
    (reputation * 0.3) + 
    (load_factor * 0.3) + 
    specialization_bonus
}
```

## Testing Requirements

### 1. Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_registration() {
        // Test agent creation with valid data
    }
    
    #[tokio::test]
    async fn test_kebab_case_validation() {
        // Test name validation rules
    }
    
    #[tokio::test]
    async fn test_workload_constraints() {
        // Test current_load constraints
    }
    
    #[tokio::test]
    async fn test_capability_search() {
        // Test finding agents by capability
    }
    
    #[tokio::test]
    async fn test_heartbeat_updates() {
        // Test heartbeat mechanism
    }
}
```

### 2. Constraint Tests
- Test kebab-case name validation
- Test load cannot exceed max
- Test reputation score bounds
- Test JSON validity for capabilities

### 3. Performance Tests
- Test capability searches with many agents
- Test work discovery queries
- Verify index usage

## Performance Considerations

1. **JSON Performance**
   - Consider normalized tables for capabilities if performance issues
   - Use JSON1 extension functions for efficient queries
   - Index frequently searched capabilities

2. **Heartbeat Updates**
   - Batch heartbeat updates if many agents
   - Consider separate heartbeat table for history

3. **Work Discovery**
   - Composite index optimizes finding available agents
   - Consider caching agent availability

## Security Considerations

1. **Name Validation**
   - Strict kebab-case prevents injection
   - Length limits prevent abuse

2. **Capability Validation**
   - Validate against known capability list
   - Prevent arbitrary capability creation

## Migration Notes

1. Requires SQLite 3.31.0+ for generated columns
2. JSON1 extension required for JSON functions
3. Consider adding:
   - Agent groups/teams
   - Skill levels per capability
   - Working hours preferences