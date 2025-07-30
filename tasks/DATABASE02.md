# DATABASE02: Create Task Messages Table

## Objective
Implement the task_messages table in SQLite with all necessary constraints, indexes, and triggers for efficient message threading and retrieval.

## Implementation Details

### 1. Create Migration File
Create `database/migrations/sqlite/004_task_messages.sql`:

```sql
-- Task Messages Table
-- Enables agent-to-agent communication about specific tasks
CREATE TABLE IF NOT EXISTS task_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    author_agent_name TEXT NOT NULL,
    message_type TEXT NOT NULL CHECK (
        message_type IN ('comment', 'question', 'update', 'blocker', 'solution', 'review', 'handoff')
    ),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    content TEXT NOT NULL CHECK (length(content) > 0 AND length(content) <= 10000),
    reply_to_message_id INTEGER,
    
    -- Foreign key constraints
    FOREIGN KEY (reply_to_message_id) REFERENCES task_messages(id) ON DELETE SET NULL,
    FOREIGN KEY (author_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_task_messages_task ON task_messages(task_code);
CREATE INDEX idx_task_messages_author ON task_messages(author_agent_name);
CREATE INDEX idx_task_messages_type ON task_messages(message_type);
CREATE INDEX idx_task_messages_created ON task_messages(created_at);
CREATE INDEX idx_task_messages_reply ON task_messages(reply_to_message_id);

-- Composite index for common queries
CREATE INDEX idx_task_messages_task_type_created ON task_messages(task_code, message_type, created_at DESC);

-- Trigger to validate task exists
CREATE TRIGGER validate_task_exists_on_message
BEFORE INSERT ON task_messages
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM tasks WHERE code = NEW.task_code) = 0
        THEN RAISE(ABORT, 'Task does not exist')
    END;
END;

-- Trigger to validate reply_to references same task
CREATE TRIGGER validate_reply_same_task
BEFORE INSERT ON task_messages
FOR EACH ROW
WHEN NEW.reply_to_message_id IS NOT NULL
BEGIN
    SELECT CASE
        WHEN (
            SELECT task_code 
            FROM task_messages 
            WHERE id = NEW.reply_to_message_id
        ) != NEW.task_code
        THEN RAISE(ABORT, 'Reply must be to message in same task')
    END;
END;

-- View for message threads
CREATE VIEW message_threads AS
WITH RECURSIVE thread_tree AS (
    -- Root messages (not replies)
    SELECT 
        id,
        task_code,
        author_agent_name,
        message_type,
        created_at,
        content,
        reply_to_message_id,
        0 as depth,
        id as thread_root_id,
        CAST(id AS TEXT) as path
    FROM task_messages
    WHERE reply_to_message_id IS NULL
    
    UNION ALL
    
    -- Replies
    SELECT 
        m.id,
        m.task_code,
        m.author_agent_name,
        m.message_type,
        m.created_at,
        m.content,
        m.reply_to_message_id,
        t.depth + 1,
        t.thread_root_id,
        t.path || '/' || CAST(m.id AS TEXT)
    FROM task_messages m
    INNER JOIN thread_tree t ON m.reply_to_message_id = t.id
)
SELECT * FROM thread_tree;

-- View for unresolved questions
CREATE VIEW unresolved_questions AS
SELECT 
    q.id,
    q.task_code,
    q.author_agent_name,
    q.created_at,
    q.content,
    (
        SELECT COUNT(*) 
        FROM task_messages a 
        WHERE a.reply_to_message_id = q.id 
          AND a.message_type = 'solution'
    ) as solution_count
FROM task_messages q
WHERE q.message_type = 'question'
  AND NOT EXISTS (
      SELECT 1 
      FROM task_messages a 
      WHERE a.reply_to_message_id = q.id 
        AND a.message_type = 'solution'
  );

-- View for active blockers
CREATE VIEW active_blockers AS
SELECT 
    b.id,
    b.task_code,
    b.author_agent_name,
    b.created_at,
    b.content,
    (
        SELECT COUNT(*) 
        FROM task_messages s 
        WHERE s.reply_to_message_id = b.id 
          AND s.message_type = 'solution'
    ) as solution_count
FROM task_messages b
WHERE b.message_type = 'blocker'
  AND NOT EXISTS (
      SELECT 1 
      FROM task_messages s 
      WHERE s.reply_to_message_id = b.id 
        AND s.message_type = 'solution'
  );
```

### 2. Create Rollback Migration
Create `database/migrations/sqlite/004_task_messages_rollback.sql`:

```sql
-- Rollback task messages migration
DROP VIEW IF EXISTS active_blockers;
DROP VIEW IF EXISTS unresolved_questions;
DROP VIEW IF EXISTS message_threads;
DROP TRIGGER IF EXISTS validate_reply_same_task;
DROP TRIGGER IF EXISTS validate_task_exists_on_message;
DROP INDEX IF EXISTS idx_task_messages_task_type_created;
DROP INDEX IF EXISTS idx_task_messages_reply;
DROP INDEX IF EXISTS idx_task_messages_created;
DROP INDEX IF EXISTS idx_task_messages_type;
DROP INDEX IF EXISTS idx_task_messages_author;
DROP INDEX IF EXISTS idx_task_messages_task;
DROP TABLE IF EXISTS task_messages;
```

### 3. Add Migration to Runner
Update `database/src/migrations.rs` to include the new migration:

```rust
pub fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // ... existing migrations ...
    
    // Run task messages migration
    sqlx::query(include_str!("../migrations/sqlite/004_task_messages.sql"))
        .execute(pool)
        .await?;
    
    Ok(())
}
```

### 4. Create Test Data Script
Create `database/scripts/test_task_messages.sql`:

```sql
-- Test data for task messages
-- Assumes tasks and agents already exist

-- Thread 1: Question and answer
INSERT INTO task_messages (task_code, author_agent_name, message_type, content)
VALUES ('IMPL-001', 'frontend-dev', 'question', 'Should we use React or Vue for this component?');

INSERT INTO task_messages (task_code, author_agent_name, message_type, content, reply_to_message_id)
VALUES ('IMPL-001', 'tech-lead', 'solution', 'Use React to maintain consistency with the rest of the codebase.', 1);

-- Thread 2: Blocker with resolution
INSERT INTO task_messages (task_code, author_agent_name, message_type, content)
VALUES ('IMPL-002', 'backend-dev', 'blocker', 'Cannot proceed - missing API specification for user endpoints');

INSERT INTO task_messages (task_code, author_agent_name, message_type, content, reply_to_message_id)
VALUES ('IMPL-002', 'api-designer', 'update', 'Working on the spec now, will have it ready in 30 minutes', 3);

INSERT INTO task_messages (task_code, author_agent_name, message_type, content, reply_to_message_id)
VALUES ('IMPL-002', 'api-designer', 'solution', 'API spec completed and available at /docs/api/users.md', 3);

-- Thread 3: Review request
INSERT INTO task_messages (task_code, author_agent_name, message_type, content)
VALUES ('IMPL-003', 'junior-dev', 'review', 'Please review my implementation of the login component');

INSERT INTO task_messages (task_code, author_agent_name, message_type, content, reply_to_message_id)
VALUES ('IMPL-003', 'senior-dev', 'comment', 'Good start! A few suggestions...', 6);

-- Handoff message
INSERT INTO task_messages (task_code, author_agent_name, message_type, content)
VALUES ('DESIGN-001', 'ui-designer', 'handoff', 'Design complete. All mockups are in Figma. Ready for implementation.');
```

## Testing Requirements

### 1. Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_creation() {
        // Test creating messages
    }
    
    #[tokio::test]
    async fn test_message_threading() {
        // Test reply chains
    }
    
    #[tokio::test]
    async fn test_invalid_task_code() {
        // Test trigger validation
    }
    
    #[tokio::test]
    async fn test_cross_task_reply_prevention() {
        // Test reply validation
    }
}
```

### 2. Query Performance Tests
- Test thread queries with deep nesting (10+ levels)
- Test message retrieval with 1000+ messages per task
- Verify index usage with EXPLAIN QUERY PLAN

### 3. View Tests
- Verify unresolved_questions view accuracy
- Verify active_blockers view accuracy
- Test message_threads view with complex hierarchies

## Performance Considerations

1. **Indexing Strategy**
   - Primary index on task_code for fast task-specific queries
   - Composite index for type+date filtering
   - Reply index for thread traversal

2. **Message Content**
   - Limited to 10KB to prevent bloat
   - Consider compression for larger deployments

3. **Thread Depth**
   - Monitor recursive CTE performance
   - Consider limiting thread depth if needed

4. **Archival Strategy**
   - Old messages could be moved to archive table
   - Consider partitioning by date for large deployments

## Migration Notes

1. This migration depends on:
   - tasks table (for task_code validation)
   - agents table (for author validation)

2. The triggers ensure referential integrity beyond foreign keys

3. Views provide convenient access to common queries

4. Consider adding FTS5 for message content search in future