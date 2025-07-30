# DATABASE06: Implement Knowledge Objects Repository

## Objective
Implement all knowledge object methods in the SQLite repository, including full-text search using FTS5 and hierarchical knowledge trees.

## Implementation Details

### 1. Add Knowledge Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for knowledge-related methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn create_knowledge_object(&self, knowledge: NewKnowledgeObject) -> Result<KnowledgeObject> {
        // Validate input
        knowledge.validate()?;
        
        // Validate task exists
        let task_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE code = ?)"
        )
        .bind(&knowledge.task_code)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !task_exists {
            return Err(TaskError::NotFound(format!("Task {} not found", knowledge.task_code)));
        }
        
        // Validate agent exists
        let agent_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(&knowledge.author_agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !agent_exists {
            return Err(TaskError::Validation(
                format!("Agent {} not registered", knowledge.author_agent_name)
            ));
        }
        
        // Validate parent if provided
        if let Some(parent_id) = knowledge.parent_knowledge_id {
            let parent_task = sqlx::query_scalar::<_, String>(
                "SELECT task_code FROM knowledge_objects WHERE id = ?"
            )
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            match parent_task {
                Some(task) if task != knowledge.task_code => {
                    return Err(TaskError::Validation(
                        "Parent knowledge must belong to same task".to_string()
                    ));
                }
                None => {
                    return Err(TaskError::NotFound(
                        format!("Parent knowledge {} not found", parent_id)
                    ));
                }
                _ => {}
            }
        }
        
        // Convert tags to JSON
        let tags_json = tags_to_json(&knowledge.tags);
        let artifacts_json = knowledge.artifacts
            .as_ref()
            .map(|a| serde_json::to_string(a).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "{}".to_string());
        
        // Insert knowledge object
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO knowledge_objects 
            (task_code, author_agent_name, knowledge_type, title, body, 
             tags, visibility, parent_knowledge_id, confidence_score, artifacts)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&knowledge.task_code)
        .bind(&knowledge.author_agent_name)
        .bind(knowledge.knowledge_type.to_string())
        .bind(&knowledge.title)
        .bind(&knowledge.body)
        .bind(&tags_json)
        .bind(knowledge.visibility.to_string())
        .bind(knowledge.parent_knowledge_id)
        .bind(knowledge.confidence_score)
        .bind(&artifacts_json)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Fetch and return created object
        self.get_knowledge_by_id(id)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created knowledge".to_string()))
    }
    
    async fn get_knowledge_objects(&self, filter: KnowledgeFilter) -> Result<Vec<KnowledgeObject>> {
        let mut query = QueryBuilder::new(
            r#"
            SELECT id, task_code, author_agent_name, knowledge_type, 
                   created_at, title, body, tags, visibility, 
                   parent_knowledge_id, confidence_score, artifacts
            FROM knowledge_objects 
            WHERE is_archived = 0
            "#
        );
        
        // Apply filters
        if let Some(task_code) = &filter.task_code {
            query.push(" AND task_code = ");
            query.push_bind(task_code);
        }
        
        if !filter.knowledge_types.is_empty() {
            query.push(" AND knowledge_type IN (");
            let mut separated = query.separated(", ");
            for kt in &filter.knowledge_types {
                separated.push_bind(kt.to_string());
            }
            query.push(")");
        }
        
        if let Some(author) = &filter.author_agent_name {
            query.push(" AND author_agent_name = ");
            query.push_bind(author);
        }
        
        if let Some(visibility) = &filter.visibility {
            query.push(" AND visibility = ");
            query.push_bind(visibility.to_string());
        }
        
        // Tag filtering using JSON
        if !filter.tags.is_empty() {
            query.push(" AND (");
            let mut first = true;
            for tag in &filter.tags {
                if !first {
                    query.push(" OR ");
                }
                query.push(" EXISTS (SELECT 1 FROM json_each(tags) WHERE value = ");
                query.push_bind(tag);
                query.push(")");
                first = false;
            }
            query.push(")");
        }
        
        if let Some(since) = filter.since {
            query.push(" AND created_at >= ");
            query.push_bind(since);
        }
        
        // Order by creation time
        query.push(" ORDER BY created_at DESC");
        
        // Apply pagination
        if let Some(limit) = filter.limit {
            query.push(" LIMIT ");
            query.push_bind(limit);
        }
        
        if let Some(offset) = filter.offset {
            query.push(" OFFSET ");
            query.push_bind(offset);
        }
        
        let knowledge_objects = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        knowledge_objects.into_iter()
            .map(|row| self.row_to_knowledge_object(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_knowledge_by_id(&self, knowledge_id: i32) -> Result<Option<KnowledgeObject>> {
        let row = sqlx::query(
            r#"
            SELECT id, task_code, author_agent_name, knowledge_type, 
                   created_at, title, body, tags, visibility, 
                   parent_knowledge_id, confidence_score, artifacts
            FROM knowledge_objects 
            WHERE id = ? AND is_archived = 0
            "#
        )
        .bind(knowledge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_knowledge_object(r)).transpose()
    }
    
    async fn search_knowledge(&self, query: KnowledgeSearchQuery) -> Result<Vec<KnowledgeObject>> {
        // Build FTS5 search query
        let fts_query = build_fts_query(&query.query);
        
        let mut sql = QueryBuilder::new(
            r#"
            SELECT DISTINCT 
                k.id, k.task_code, k.author_agent_name, k.knowledge_type,
                k.created_at, k.title, k.body, k.tags, k.visibility,
                k.parent_knowledge_id, k.confidence_score, k.artifacts,
                rank
            FROM knowledge_objects k
            INNER JOIN knowledge_search s ON k.id = s.knowledge_id
            WHERE knowledge_search MATCH 
            "#
        );
        
        sql.push_bind(&fts_query);
        sql.push(" AND k.is_archived = 0");
        
        // Filter by task codes
        if let Some(task_codes) = &query.task_codes {
            if !task_codes.is_empty() {
                sql.push(" AND k.task_code IN (");
                let mut separated = sql.separated(", ");
                for code in task_codes {
                    separated.push_bind(code);
                }
                sql.push(")");
            }
        }
        
        // Filter by knowledge types
        if !query.knowledge_types.is_empty() {
            sql.push(" AND k.knowledge_type IN (");
            let mut separated = sql.separated(", ");
            for kt in &query.knowledge_types {
                separated.push_bind(kt.to_string());
            }
            sql.push(")");
        }
        
        // Filter by tags
        if !query.tags.is_empty() {
            sql.push(" AND (");
            let mut first = true;
            for tag in &query.tags {
                if !first {
                    sql.push(" OR ");
                }
                sql.push(" EXISTS (SELECT 1 FROM json_each(k.tags) WHERE value = ");
                sql.push_bind(tag);
                sql.push(")");
                first = false;
            }
            sql.push(")");
        }
        
        // Filter by visibility
        if let Some(visibility) = &query.visibility_filter {
            sql.push(" AND k.visibility = ");
            sql.push_bind(visibility.to_string());
        }
        
        // Order by relevance (FTS5 rank)
        sql.push(" ORDER BY rank");
        
        if let Some(limit) = query.limit {
            sql.push(" LIMIT ");
            sql.push_bind(limit);
        }
        
        let results = sql.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        results.into_iter()
            .map(|row| self.row_to_knowledge_object(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_knowledge_tree(&self, root_id: i32) -> Result<Vec<KnowledgeObject>> {
        // Recursive CTE to get entire tree
        let tree = sqlx::query(
            r#"
            WITH RECURSIVE tree AS (
                -- Base case: the root knowledge object
                SELECT id, task_code, author_agent_name, knowledge_type,
                       created_at, title, body, tags, visibility,
                       parent_knowledge_id, confidence_score, artifacts,
                       0 as depth
                FROM knowledge_objects
                WHERE id = ? AND is_archived = 0
                
                UNION ALL
                
                -- Recursive case: all children
                SELECT k.id, k.task_code, k.author_agent_name, k.knowledge_type,
                       k.created_at, k.title, k.body, k.tags, k.visibility,
                       k.parent_knowledge_id, k.confidence_score, k.artifacts,
                       t.depth + 1
                FROM knowledge_objects k
                INNER JOIN tree t ON k.parent_knowledge_id = t.id
                WHERE k.is_archived = 0
            )
            SELECT id, task_code, author_agent_name, knowledge_type,
                   created_at, title, body, tags, visibility,
                   parent_knowledge_id, confidence_score, artifacts
            FROM tree
            ORDER BY depth, created_at
            "#
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        tree.into_iter()
            .map(|row| self.row_to_knowledge_object(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn update_knowledge_tags(&self, knowledge_id: i32, tags: Vec<String>) -> Result<()> {
        // Validate tags
        if tags.len() > 20 {
            return Err(TaskError::Validation(
                "Cannot have more than 20 tags".to_string()
            ));
        }
        
        let tags_json = tags_to_json(&tags);
        
        let affected = sqlx::query(
            "UPDATE knowledge_objects SET tags = ? WHERE id = ? AND is_archived = 0"
        )
        .bind(&tags_json)
        .bind(knowledge_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(
                format!("Knowledge object {} not found", knowledge_id)
            ));
        }
        
        Ok(())
    }
    
    async fn get_related_knowledge(&self, knowledge_id: i32, limit: i32) -> Result<Vec<KnowledgeObject>> {
        // Get related knowledge based on:
        // 1. Same parent
        // 2. Same task and similar tags
        // 3. Same knowledge type
        
        let base_knowledge = self.get_knowledge_by_id(knowledge_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Knowledge {} not found", knowledge_id)))?;
        
        let related = sqlx::query(
            r#"
            SELECT DISTINCT
                k.id, k.task_code, k.author_agent_name, k.knowledge_type,
                k.created_at, k.title, k.body, k.tags, k.visibility,
                k.parent_knowledge_id, k.confidence_score, k.artifacts,
                -- Scoring for relevance
                CASE
                    WHEN k.parent_knowledge_id = ? THEN 3
                    WHEN k.parent_knowledge_id = ? AND ? IS NOT NULL THEN 3
                    WHEN k.task_code = ? AND k.knowledge_type = ? THEN 2
                    WHEN k.task_code = ? THEN 1
                    ELSE 0
                END as relevance_score
            FROM knowledge_objects k
            WHERE k.id != ?
              AND k.is_archived = 0
              AND (
                  k.parent_knowledge_id = ? OR
                  (k.parent_knowledge_id = ? AND ? IS NOT NULL) OR
                  k.task_code = ?
              )
            ORDER BY relevance_score DESC, k.created_at DESC
            LIMIT ?
            "#
        )
        .bind(knowledge_id)
        .bind(&base_knowledge.parent_knowledge_id)
        .bind(&base_knowledge.parent_knowledge_id)
        .bind(&base_knowledge.task_code)
        .bind(base_knowledge.knowledge_type.to_string())
        .bind(&base_knowledge.task_code)
        .bind(knowledge_id)
        .bind(knowledge_id)
        .bind(&base_knowledge.parent_knowledge_id)
        .bind(&base_knowledge.parent_knowledge_id)
        .bind(&base_knowledge.task_code)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        related.into_iter()
            .map(|row| self.row_to_knowledge_object(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn count_knowledge_by_type(&self, task_code: &str) -> Result<KnowledgeCountByType> {
        let counts = sqlx::query(
            r#"
            SELECT 
                knowledge_type,
                visibility,
                COUNT(*) as count
            FROM knowledge_objects
            WHERE task_code = ? AND is_archived = 0
            GROUP BY knowledge_type, visibility
            "#
        )
        .bind(task_code)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut result = KnowledgeCountByType::default();
        
        for row in counts {
            let type_str: String = row.get("knowledge_type");
            let visibility_str: String = row.get("visibility");
            let count: i32 = row.get("count");
            
            result.total += count;
            
            // Count by visibility
            match visibility_str.as_str() {
                "public" => result.public_count += count,
                "team" => result.team_count += count,
                "private" => result.private_count += count,
                _ => {}
            }
            
            // Count by type
            if let Ok(kt) = KnowledgeType::try_from(type_str.as_str()) {
                *result.by_type.entry(kt).or_insert(0) += count;
            }
        }
        
        Ok(result)
    }
    
    async fn archive_knowledge(&self, knowledge_id: i32) -> Result<()> {
        let affected = sqlx::query(
            "UPDATE knowledge_objects SET is_archived = 1 WHERE id = ?"
        )
        .bind(knowledge_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?
        .rows_affected();
        
        if affected == 0 {
            return Err(TaskError::NotFound(
                format!("Knowledge object {} not found", knowledge_id)
            ));
        }
        
        Ok(())
    }
}
```

### 2. Add Helper Method for Row Conversion
```rust
impl SqliteTaskRepository {
    fn row_to_knowledge_object(&self, row: SqliteRow) -> Result<KnowledgeObject> {
        let type_str: String = row.get("knowledge_type");
        let knowledge_type = KnowledgeType::try_from(type_str.as_str())?;
        
        let visibility_str: String = row.get("visibility");
        let visibility = Visibility::try_from(visibility_str.as_str())?;
        
        let tags_json: String = row.get("tags");
        let tags = parse_tags(Some(&tags_json));
        
        let artifacts_json: String = row.get("artifacts");
        let artifacts = serde_json::from_str(&artifacts_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        Ok(KnowledgeObject {
            id: row.get("id"),
            task_code: row.get("task_code"),
            author_agent_name: row.get("author_agent_name"),
            knowledge_type,
            created_at: row.get("created_at"),
            title: row.get("title"),
            body: row.get("body"),
            tags,
            visibility,
            parent_knowledge_id: row.get("parent_knowledge_id"),
            confidence_score: row.get("confidence_score"),
            artifacts,
        })
    }
}
```

### 3. Create Knowledge Helper Module
Create `database/src/knowledge_helpers.rs`:

```rust
use crate::error::Result;
use core::models::{KnowledgeType, Visibility};

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

/// Check if knowledge should be visible to agent
pub fn is_visible_to_agent(
    visibility: &Visibility,
    author: &str,
    requesting_agent: &str,
) -> bool {
    match visibility {
        Visibility::Public => true,
        Visibility::Team => true, // All agents are on same team
        Visibility::Private => author == requesting_agent,
    }
}
```

## Files to Modify
- `database/src/sqlite.rs` - Add knowledge method implementations
- `database/src/knowledge_helpers.rs` - New file with helper functions
- `database/src/lib.rs` - Export knowledge helpers module

## Testing Requirements
1. Test knowledge creation with all field combinations
2. Test parent-child relationships
3. Test FTS5 search with various queries
4. Test tag filtering with JSON queries
5. Test visibility rules enforcement
6. Test knowledge tree retrieval
7. Test related knowledge algorithm

## Performance Considerations
1. FTS5 index maintenance on insert/update
2. JSON tag queries may need optimization for large datasets
3. Knowledge tree queries use recursive CTEs - monitor depth
4. Consider caching frequently accessed knowledge objects

## Security Considerations
1. Always enforce visibility rules in queries
2. Validate all JSON inputs
3. Escape FTS5 search queries properly
4. Limit tree depth to prevent DOS