# DATABASE03: Create Knowledge Objects Table

## Objective
Implement the knowledge_objects table with FTS5 full-text search, proper indexing, and support for hierarchical knowledge structures.

## Implementation Details

### 1. Create Migration File
Create `database/migrations/sqlite/005_knowledge_objects.sql`:

```sql
-- Knowledge Objects Table
-- Stores contextual information, decisions, and artifacts for tasks
CREATE TABLE IF NOT EXISTS knowledge_objects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_code TEXT NOT NULL,
    author_agent_name TEXT NOT NULL,
    knowledge_type TEXT NOT NULL CHECK (
        knowledge_type IN ('note', 'decision', 'question', 'answer', 'handoff', 
                          'step_output', 'blocker', 'resolution', 'artifact')
    ),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    title TEXT NOT NULL CHECK (length(title) > 0 AND length(title) <= 200),
    body TEXT NOT NULL CHECK (length(body) > 0 AND length(body) <= 50000),
    tags TEXT, -- JSON array of tags
    visibility TEXT NOT NULL DEFAULT 'public' CHECK (
        visibility IN ('public', 'team', 'private')
    ),
    parent_knowledge_id INTEGER,
    confidence_score REAL CHECK (
        confidence_score IS NULL OR 
        (confidence_score >= 0.0 AND confidence_score <= 1.0)
    ),
    artifacts TEXT, -- JSON object with links, files, etc.
    is_archived BOOLEAN DEFAULT 0,
    
    -- Foreign key constraints
    FOREIGN KEY (parent_knowledge_id) REFERENCES knowledge_objects(id) ON DELETE SET NULL,
    FOREIGN KEY (author_agent_name) REFERENCES agents(name) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_knowledge_task ON knowledge_objects(task_code);
CREATE INDEX idx_knowledge_author ON knowledge_objects(author_agent_name);
CREATE INDEX idx_knowledge_type ON knowledge_objects(knowledge_type);
CREATE INDEX idx_knowledge_visibility ON knowledge_objects(visibility);
CREATE INDEX idx_knowledge_created ON knowledge_objects(created_at DESC);
CREATE INDEX idx_knowledge_parent ON knowledge_objects(parent_knowledge_id);
CREATE INDEX idx_knowledge_archived ON knowledge_objects(is_archived);

-- Composite indexes for common queries
CREATE INDEX idx_knowledge_task_type_vis ON knowledge_objects(task_code, knowledge_type, visibility)
    WHERE is_archived = 0;
CREATE INDEX idx_knowledge_task_created ON knowledge_objects(task_code, created_at DESC)
    WHERE is_archived = 0;

-- JSON index for tags (if SQLite has JSON1 extension)
CREATE INDEX idx_knowledge_tags ON knowledge_objects(json_extract(tags, '$'));

-- FTS5 Virtual Table for Full-Text Search
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_search USING fts5(
    knowledge_id UNINDEXED,
    title,
    body,
    tags,
    tokenize = 'porter unicode61',
    content = 'knowledge_objects',
    content_rowid = 'id'
);

-- Triggers to maintain FTS index
CREATE TRIGGER knowledge_insert_fts 
AFTER INSERT ON knowledge_objects 
WHEN NEW.is_archived = 0
BEGIN
    INSERT INTO knowledge_search(knowledge_id, title, body, tags) 
    VALUES (NEW.id, NEW.title, NEW.body, NEW.tags);
END;

CREATE TRIGGER knowledge_delete_fts 
AFTER DELETE ON knowledge_objects 
BEGIN
    DELETE FROM knowledge_search WHERE knowledge_id = OLD.id;
END;

CREATE TRIGGER knowledge_update_fts 
AFTER UPDATE ON knowledge_objects 
BEGIN
    DELETE FROM knowledge_search WHERE knowledge_id = OLD.id;
    INSERT INTO knowledge_search(knowledge_id, title, body, tags) 
    SELECT NEW.id, NEW.title, NEW.body, NEW.tags
    WHERE NEW.is_archived = 0;
END;

-- Trigger to validate task exists
CREATE TRIGGER validate_task_exists_on_knowledge
BEFORE INSERT ON knowledge_objects
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM tasks WHERE code = NEW.task_code) = 0
        THEN RAISE(ABORT, 'Task does not exist')
    END;
END;

-- Trigger to validate parent knowledge belongs to same task
CREATE TRIGGER validate_parent_same_task
BEFORE INSERT ON knowledge_objects
FOR EACH ROW
WHEN NEW.parent_knowledge_id IS NOT NULL
BEGIN
    SELECT CASE
        WHEN (
            SELECT task_code 
            FROM knowledge_objects 
            WHERE id = NEW.parent_knowledge_id
        ) != NEW.task_code
        THEN RAISE(ABORT, 'Parent knowledge must belong to same task')
    END;
END;

-- View for knowledge trees
CREATE VIEW knowledge_tree AS
WITH RECURSIVE tree AS (
    -- Root nodes
    SELECT 
        id,
        task_code,
        author_agent_name,
        knowledge_type,
        title,
        parent_knowledge_id,
        0 as depth,
        CAST(id AS TEXT) as path,
        created_at,
        visibility,
        confidence_score
    FROM knowledge_objects
    WHERE parent_knowledge_id IS NULL AND is_archived = 0
    
    UNION ALL
    
    -- Child nodes
    SELECT 
        k.id,
        k.task_code,
        k.author_agent_name,
        k.knowledge_type,
        k.title,
        k.parent_knowledge_id,
        t.depth + 1,
        t.path || '/' || CAST(k.id AS TEXT),
        k.created_at,
        k.visibility,
        k.confidence_score
    FROM knowledge_objects k
    INNER JOIN tree t ON k.parent_knowledge_id = t.id
    WHERE k.is_archived = 0
)
SELECT * FROM tree;

-- View for decisions with high confidence
CREATE VIEW high_confidence_decisions AS
SELECT 
    id,
    task_code,
    author_agent_name,
    title,
    body,
    confidence_score,
    created_at
FROM knowledge_objects
WHERE knowledge_type = 'decision'
  AND confidence_score >= 0.8
  AND is_archived = 0
  AND visibility IN ('public', 'team')
ORDER BY confidence_score DESC, created_at DESC;

-- View for unanswered questions
CREATE VIEW unanswered_questions AS
SELECT 
    q.id,
    q.task_code,
    q.author_agent_name,
    q.title,
    q.body,
    q.created_at,
    (
        SELECT COUNT(*) 
        FROM knowledge_objects a 
        WHERE a.parent_knowledge_id = q.id 
          AND a.knowledge_type = 'answer'
    ) as answer_count
FROM knowledge_objects q
WHERE q.knowledge_type = 'question'
  AND q.is_archived = 0
  AND NOT EXISTS (
      SELECT 1 
      FROM knowledge_objects a 
      WHERE a.parent_knowledge_id = q.id 
        AND a.knowledge_type = 'answer'
  );
```

### 2. Create Rollback Migration
Create `database/migrations/sqlite/005_knowledge_objects_rollback.sql`:

```sql
-- Rollback knowledge objects migration
DROP VIEW IF EXISTS unanswered_questions;
DROP VIEW IF EXISTS high_confidence_decisions;
DROP VIEW IF EXISTS knowledge_tree;
DROP TRIGGER IF EXISTS validate_parent_same_task;
DROP TRIGGER IF EXISTS validate_task_exists_on_knowledge;
DROP TRIGGER IF EXISTS knowledge_update_fts;
DROP TRIGGER IF EXISTS knowledge_delete_fts;
DROP TRIGGER IF EXISTS knowledge_insert_fts;
DROP TABLE IF EXISTS knowledge_search;
DROP INDEX IF EXISTS idx_knowledge_tags;
DROP INDEX IF EXISTS idx_knowledge_task_created;
DROP INDEX IF EXISTS idx_knowledge_task_type_vis;
DROP INDEX IF EXISTS idx_knowledge_archived;
DROP INDEX IF EXISTS idx_knowledge_parent;
DROP INDEX IF EXISTS idx_knowledge_created;
DROP INDEX IF EXISTS idx_knowledge_visibility;
DROP INDEX IF EXISTS idx_knowledge_type;
DROP INDEX IF EXISTS idx_knowledge_author;
DROP INDEX IF EXISTS idx_knowledge_task;
DROP TABLE IF EXISTS knowledge_objects;
```

### 3. Create Helper Functions in Database Module
In `database/src/knowledge.rs`:

```rust
/// Parse tags from JSON string
pub fn parse_tags(tags_json: Option<&str>) -> Vec<String> {
    tags_json
        .and_then(|json| serde_json::from_str::<Vec<String>>(json).ok())
        .unwrap_or_default()
}

/// Convert tags to JSON string
pub fn tags_to_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

/// Build FTS5 search query with proper escaping
pub fn build_fts_query(search_term: &str) -> String {
    // Escape special FTS5 characters
    let escaped = search_term
        .replace('"', "\"\"")
        .replace('*', "")
        .replace('(', "")
        .replace(')', "");
    
    // Use prefix search for each word
    escaped.split_whitespace()
        .map(|word| format!("{}*", word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate relevance score for search results
pub fn calculate_relevance(
    title_matches: bool,
    body_matches: bool,
    tag_matches: bool,
    confidence_score: Option<f64>,
) -> f64 {
    let mut score = 0.0;
    
    if title_matches { score += 0.4; }
    if body_matches { score += 0.3; }
    if tag_matches { score += 0.2; }
    
    if let Some(confidence) = confidence_score {
        score += confidence * 0.1;
    }
    
    score
}
```

### 4. Create Test Data Script
Create `database/scripts/test_knowledge_objects.sql`:

```sql
-- Test data for knowledge objects
-- Assumes tasks and agents already exist

-- Decision with high confidence
INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, confidence_score, tags)
VALUES (
    'ARCH-001',
    'tech-lead',
    'decision',
    'Use Event Sourcing for Audit Trail',
    'After analyzing requirements, event sourcing provides the best solution for maintaining a complete audit trail.',
    0.95,
    '["architecture", "event-sourcing", "audit", "decision"]'
);

-- Question and answer tree
INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, tags)
VALUES (
    'IMPL-001',
    'junior-dev',
    'question',
    'How should we handle authentication?',
    'What is the recommended approach for implementing user authentication in this system?',
    '["authentication", "security", "question"]'
);

INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, parent_knowledge_id, confidence_score, tags)
VALUES (
    'IMPL-001',
    'security-expert',
    'answer',
    'Use OAuth2 with JWT tokens',
    'Implement OAuth2 flow with JWT tokens for stateless authentication. This provides security and scalability.',
    2,
    0.9,
    '["authentication", "oauth2", "jwt", "answer"]'
);

-- Step outputs
INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, artifacts)
VALUES (
    'IMPL-002',
    'backend-dev',
    'step_output',
    'Database Schema Created',
    'Successfully created all tables and indexes for the user management system.',
    '{"migration_files": ["001_users.sql", "002_roles.sql"], "schema_diagram": "/docs/db-schema.png"}'
);

-- Private note
INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, visibility)
VALUES (
    'DEBUG-001',
    'senior-dev',
    'note',
    'Performance Issue Investigation',
    'The slow query is caused by missing index on user_sessions.last_active column.',
    'private'
);

-- Blocker and resolution
INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, tags)
VALUES (
    'IMPL-003',
    'frontend-dev',
    'blocker',
    'CORS Issues with API',
    'Getting CORS errors when calling API from localhost:3000. Unable to proceed with integration.',
    '["cors", "api", "blocker", "integration"]'
);

INSERT INTO knowledge_objects (task_code, author_agent_name, knowledge_type, title, body, parent_knowledge_id, tags)
VALUES (
    'IMPL-003',
    'backend-dev',
    'resolution',
    'CORS Configuration Updated',
    'Added localhost:3000 to allowed origins in API gateway configuration. Issue resolved.',
    6,
    '["cors", "api", "resolution", "configuration"]'
);
```

## Testing Requirements

### 1. Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_knowledge_creation() {
        // Test creating knowledge objects
    }
    
    #[tokio::test]
    async fn test_knowledge_hierarchy() {
        // Test parent-child relationships
    }
    
    #[tokio::test]
    async fn test_fts_search() {
        // Test full-text search functionality
    }
    
    #[tokio::test]
    async fn test_visibility_filtering() {
        // Test visibility rules
    }
    
    #[tokio::test]
    async fn test_tag_filtering() {
        // Test JSON tag queries
    }
}
```

### 2. FTS5 Search Tests
- Test phrase searches
- Test prefix searches
- Test boolean operators
- Test ranking/relevance

### 3. Performance Tests
- Test search performance with 10K+ knowledge objects
- Test tree queries with deep hierarchies
- Verify index usage

## Performance Considerations

1. **FTS5 Optimization**
   - Use porter stemmer for better search
   - Consider custom tokenizer for technical terms
   - Monitor index size growth

2. **Tag Indexing**
   - JSON functions require SQLite 3.38.0+
   - Consider normalized tag table for older versions

3. **Tree Queries**
   - Limit depth to prevent excessive recursion
   - Consider materialized path for very deep trees

4. **Archival Strategy**
   - Soft delete with is_archived flag
   - Exclude archived from FTS index
   - Periodic cleanup of old archived items

## Security Considerations

1. **Visibility Enforcement**
   - Always filter by visibility in queries
   - Private knowledge only visible to author
   - Team knowledge visible to all agents

2. **Content Validation**
   - Enforce maximum content length
   - Validate JSON structures
   - Sanitize search queries

## Migration Notes

1. Requires SQLite with:
   - FTS5 extension
   - JSON1 extension (optional but recommended)

2. FTS5 configuration:
   - Porter tokenizer for stemming
   - Unicode support for international text

3. Consider adding:
   - Spell correction for search
   - Synonym support
   - Faceted search by tags/type