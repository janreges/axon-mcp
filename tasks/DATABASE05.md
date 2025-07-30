# DATABASE05: Implement Task Messages Repository

## Objective
Implement all task message methods in the SQLite repository, providing full message functionality for agent communication.

## Implementation Details

### 1. Add Message Methods to SqliteTaskRepository
In `database/src/sqlite.rs`, add implementations for all message-related trait methods:

```rust
impl TaskRepository for SqliteTaskRepository {
    // ... existing implementations ...
    
    async fn add_task_message(&self, message: NewTaskMessage) -> Result<TaskMessage> {
        // Validate message
        message.validate()?;
        
        // Validate task exists
        let task_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE code = ?)"
        )
        .bind(&message.task_code)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !task_exists {
            return Err(TaskError::NotFound(format!("Task {} not found", message.task_code)));
        }
        
        // Validate agent exists
        let agent_exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM agents WHERE name = ?)"
        )
        .bind(&message.author_agent_name)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        if !agent_exists {
            return Err(TaskError::Validation(
                format!("Agent {} not registered", message.author_agent_name)
            ));
        }
        
        // Validate reply_to if provided
        if let Some(reply_to_id) = message.reply_to_message_id {
            let reply_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM task_messages WHERE id = ? AND task_code = ?)"
            )
            .bind(reply_to_id)
            .bind(&message.task_code)
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
            
            if !reply_exists {
                return Err(TaskError::Validation(
                    format!("Reply to message {} not found in task", reply_to_id)
                ));
            }
        }
        
        // Insert message
        let id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO task_messages 
            (task_code, author_agent_name, message_type, content, reply_to_message_id)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#
        )
        .bind(&message.task_code)
        .bind(&message.author_agent_name)
        .bind(message.message_type.to_string())
        .bind(&message.content)
        .bind(message.reply_to_message_id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        // Fetch and return the created message
        self.get_message_by_id(id)
            .await?
            .ok_or_else(|| TaskError::Database("Failed to fetch created message".to_string()))
    }
    
    async fn get_task_messages(&self, filter: MessageFilter) -> Result<Vec<TaskMessage>> {
        let mut query = QueryBuilder::new(
            "SELECT id, task_code, author_agent_name, message_type, created_at, content, reply_to_message_id 
             FROM task_messages WHERE 1=1"
        );
        
        // Apply filters
        if let Some(task_code) = &filter.task_code {
            query.push(" AND task_code = ");
            query.push_bind(task_code);
        }
        
        if !filter.message_types.is_empty() {
            query.push(" AND message_type IN (");
            let mut separated = query.separated(", ");
            for msg_type in &filter.message_types {
                separated.push_bind(msg_type.to_string());
            }
            query.push(")");
        }
        
        if let Some(since) = filter.since {
            query.push(" AND created_at >= ");
            query.push_bind(since);
        }
        
        if let Some(author) = &filter.author_agent_name {
            query.push(" AND author_agent_name = ");
            query.push_bind(author);
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
        
        let messages = query.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        messages.into_iter()
            .map(|row| self.row_to_task_message(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_message_by_id(&self, message_id: i32) -> Result<Option<TaskMessage>> {
        let row = sqlx::query(
            "SELECT id, task_code, author_agent_name, message_type, created_at, content, reply_to_message_id 
             FROM task_messages WHERE id = ?"
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        row.map(|r| self.row_to_task_message(r)).transpose()
    }
    
    async fn search_task_messages(&self, query: MessageSearchQuery) -> Result<Vec<TaskMessage>> {
        let mut sql = QueryBuilder::new(
            "SELECT DISTINCT m.id, m.task_code, m.author_agent_name, m.message_type, 
                    m.created_at, m.content, m.reply_to_message_id
             FROM task_messages m
             WHERE m.content LIKE "
        );
        
        // Add search pattern
        let search_pattern = format!("%{}%", query.query);
        sql.push_bind(search_pattern);
        
        // Filter by task codes if provided
        if let Some(task_codes) = &query.task_codes {
            if !task_codes.is_empty() {
                sql.push(" AND m.task_code IN (");
                let mut separated = sql.separated(", ");
                for code in task_codes {
                    separated.push_bind(code);
                }
                sql.push(")");
            }
        }
        
        // Filter by message types
        if !query.message_types.is_empty() {
            sql.push(" AND m.message_type IN (");
            let mut separated = sql.separated(", ");
            for msg_type in &query.message_types {
                separated.push_bind(msg_type.to_string());
            }
            sql.push(")");
        }
        
        sql.push(" ORDER BY m.created_at DESC");
        
        if let Some(limit) = query.limit {
            sql.push(" LIMIT ");
            sql.push_bind(limit);
        }
        
        let messages = sql.build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;
        
        messages.into_iter()
            .map(|row| self.row_to_task_message(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn get_message_thread(&self, message_id: i32) -> Result<Vec<TaskMessage>> {
        // Recursive CTE to get all messages in a thread
        let messages = sqlx::query(
            r#"
            WITH RECURSIVE thread AS (
                -- Base case: the original message
                SELECT id, task_code, author_agent_name, message_type, 
                       created_at, content, reply_to_message_id, 0 as depth
                FROM task_messages 
                WHERE id = ?
                
                UNION ALL
                
                -- Recursive case: all replies
                SELECT m.id, m.task_code, m.author_agent_name, m.message_type,
                       m.created_at, m.content, m.reply_to_message_id, t.depth + 1
                FROM task_messages m
                INNER JOIN thread t ON m.reply_to_message_id = t.id
            )
            SELECT id, task_code, author_agent_name, message_type, 
                   created_at, content, reply_to_message_id
            FROM thread
            ORDER BY depth, created_at
            "#
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        messages.into_iter()
            .map(|row| self.row_to_task_message(row))
            .collect::<Result<Vec<_>>>()
    }
    
    async fn count_task_messages(&self, task_code: &str) -> Result<MessageCountByType> {
        let counts = sqlx::query(
            r#"
            SELECT 
                message_type,
                COUNT(*) as count
            FROM task_messages
            WHERE task_code = ?
            GROUP BY message_type
            "#
        )
        .bind(task_code)
        .fetch_all(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;
        
        let mut result = MessageCountByType::default();
        
        for row in counts {
            let msg_type_str: String = row.get("message_type");
            let count: i32 = row.get("count");
            
            if let Ok(msg_type) = MessageType::try_from(msg_type_str.as_str()) {
                result.by_type.insert(msg_type, count);
                result.total += count;
                
                // Count unresolved questions and active blockers
                match msg_type {
                    MessageType::Question => {
                        // Check if questions have answers
                        let unanswered = sqlx::query_scalar::<_, i32>(
                            r#"
                            SELECT COUNT(*)
                            FROM task_messages q
                            WHERE q.task_code = ? 
                              AND q.message_type = 'question'
                              AND NOT EXISTS (
                                  SELECT 1 FROM task_messages a 
                                  WHERE a.reply_to_message_id = q.id 
                                    AND a.message_type = 'solution'
                              )
                            "#
                        )
                        .bind(task_code)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(sqlx_error_to_task_error)?;
                        
                        result.unresolved_questions = unanswered;
                    }
                    MessageType::Blocker => {
                        // Check if blockers have resolutions
                        let active = sqlx::query_scalar::<_, i32>(
                            r#"
                            SELECT COUNT(*)
                            FROM task_messages b
                            WHERE b.task_code = ? 
                              AND b.message_type = 'blocker'
                              AND NOT EXISTS (
                                  SELECT 1 FROM task_messages s 
                                  WHERE s.reply_to_message_id = b.id 
                                    AND s.message_type = 'solution'
                              )
                            "#
                        )
                        .bind(task_code)
                        .fetch_one(&self.pool)
                        .await
                        .map_err(sqlx_error_to_task_error)?;
                        
                        result.active_blockers = active;
                    }
                    _ => {}
                }
            }
        }
        
        Ok(result)
    }
    
    async fn mark_messages_read(&self, task_code: &str, agent_name: &str) -> Result<()> {
        // For future implementation when we add read tracking
        // For now, this is a no-op
        Ok(())
    }
    
    async fn get_unread_count(&self, agent_name: &str) -> Result<i32> {
        // For future implementation when we add read tracking
        // For now, return 0
        Ok(0)
    }
}
```

### 2. Add Helper Method for Row Conversion
```rust
impl SqliteTaskRepository {
    fn row_to_task_message(&self, row: SqliteRow) -> Result<TaskMessage> {
        let message_type_str: String = row.get("message_type");
        let message_type = MessageType::try_from(message_type_str.as_str())?;
        
        Ok(TaskMessage {
            id: row.get("id"),
            task_code: row.get("task_code"),
            author_agent_name: row.get("author_agent_name"),
            message_type,
            created_at: row.get("created_at"),
            content: row.get("content"),
            reply_to_message_id: row.get("reply_to_message_id"),
        })
    }
}
```

### 3. Add Database Error Handling
Update error conversion if needed:

```rust
fn sqlx_error_to_task_error(err: sqlx::Error) -> TaskError {
    match &err {
        sqlx::Error::Database(db_err) => {
            let message = db_err.message();
            
            // Check for foreign key violations
            if message.contains("FOREIGN KEY") {
                if message.contains("agents") {
                    return TaskError::Validation("Agent not found".to_string());
                }
                if message.contains("task_messages") {
                    return TaskError::Validation("Reply to message not found".to_string());
                }
            }
            
            TaskError::Database(message.to_string())
        }
        _ => TaskError::Database(err.to_string()),
    }
}
```

## Files to Modify
- `database/src/sqlite.rs` - Add all message method implementations
- `database/src/common.rs` - Add any shared query builders if needed

## Testing Requirements
1. Test message creation with valid and invalid data
2. Test reply threading
3. Test message filtering and pagination
4. Test search functionality
5. Test message counting with different types
6. Test concurrent message creation
7. Test foreign key constraints (invalid task/agent)

## Performance Considerations
1. Message search might benefit from FTS5 in the future
2. Thread queries use recursive CTEs - test with deep threads
3. Consider adding index on content for search if needed
4. Monitor query performance with large message volumes