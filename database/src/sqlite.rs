use crate::common::{
    row_to_task, row_to_task_message, sqlx_error_to_task_error, state_to_string, string_to_state,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{migrate::MigrateDatabase, Row, Sqlite, SqlitePool};
use std::collections::HashMap;
use task_core::{
    error::{Result, TaskError},
    models::{NewTask, Task, TaskFilter, TaskMessage, TaskState, UpdateTask},
    repository::{RepositoryStats, TaskMessageRepository, TaskRepository},
};

/// SQLite implementation of the TaskRepository trait
///
/// This implementation provides high-performance task persistence using SQLite
/// with connection pooling, prepared statements, and comprehensive error handling.
#[derive(Debug, Clone)]
pub struct SqliteTaskRepository {
    pool: SqlitePool,
}

impl SqliteTaskRepository {
    /// Create a new SQLite repository with the given database URL
    ///
    /// # Arguments
    /// * `database_url` - SQLite database URL (file path or `:memory:`)
    ///
    /// # Returns
    /// * `Ok(SqliteTaskRepository)` - Successfully connected repository
    /// * `Err(TaskError::Database)` - If connection fails
    ///
    /// # Examples
    /// ```rust,no_run
    /// use database::SqliteTaskRepository;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // In-memory database for testing
    /// let repo = SqliteTaskRepository::new(":memory:").await?;
    ///
    /// // File-based database
    /// let repo = SqliteTaskRepository::new("sqlite:///tmp/tasks.db").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> Result<Self> {
        // Handle different database URL formats
        let db_url = if database_url.starts_with(":memory:") {
            // For in-memory databases, use the exact format
            database_url.to_string()
        } else if database_url.starts_with("sqlite://") {
            database_url.to_string()
        } else {
            format!("sqlite://{database_url}")
        };

        // Create database if it doesn't exist (for file-based databases)
        if !db_url.contains(":memory:") && !Sqlite::database_exists(&db_url).await.unwrap_or(false)
        {
            match Sqlite::create_database(&db_url).await {
                Ok(_) => tracing::info!("Database created successfully"),
                Err(error) => {
                    tracing::error!("Error creating database: {}", error);
                    return Err(TaskError::Database(format!(
                        "Failed to create database: {error}"
                    )));
                }
            }
        }

        // Create connection pool with optimal settings
        let connect_options = if db_url.contains(":memory:") {
            // For in-memory databases, use a simpler connection
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&db_url)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Memory)
                .busy_timeout(std::time::Duration::from_secs(5))
                .foreign_keys(true)
        } else {
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(db_url.replace("sqlite://", ""))
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .busy_timeout(std::time::Duration::from_secs(5))
                .foreign_keys(true)
        };

        let pool = SqlitePool::connect_with(connect_options)
            .await
            .map_err(sqlx_error_to_task_error)?;

        Ok(Self { pool })
    }

    /// Run database migrations
    ///
    /// This method applies all pending migrations to bring the database schema
    /// up to date. It should be called after creating a new repository instance.
    ///
    /// # Returns
    /// * `Ok(())` - Migrations completed successfully
    /// * `Err(TaskError::Database)` - If migration fails
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations/sqlite")
            .run(&self.pool)
            .await
            .map_err(|e| TaskError::Database(format!("Migration failed: {e}")))?;

        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Get access to the underlying database pool for custom operations
    ///
    /// This method is primarily intended for testing scenarios where
    /// direct SQL execution is needed.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl TaskRepository for SqliteTaskRepository {
    async fn create(&self, task: NewTask) -> Result<Task> {
        // Validate input data
        if task.code.trim().is_empty() {
            return Err(TaskError::empty_field("code"));
        }
        if task.name.trim().is_empty() {
            return Err(TaskError::empty_field("name"));
        }
        if task.description.trim().is_empty() {
            return Err(TaskError::empty_field("description"));
        }
        if let Some(ref owner) = task.owner_agent_name {
            if owner.trim().is_empty() {
                return Err(TaskError::empty_field("owner_agent_name"));
            }
        }

        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO tasks (code, name, description, owner_agent_name, state, inserted_at)
            VALUES (?, ?, ?, ?, ?, ?)
            RETURNING id, code, name, description, owner_agent_name, state, inserted_at, done_at, claimed_at
            "#,
        )
        .bind(&task.code)
        .bind(&task.name)
        .bind(&task.description)
        .bind(&task.owner_agent_name)
        .bind(state_to_string(TaskState::Created))
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        row_to_task(&row)
    }

    async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task> {
        // Check if task exists first
        let existing = self.get_by_id(id).await?;
        if existing.is_none() {
            return Err(TaskError::not_found_id(id));
        }

        // Build dynamic update query using QueryBuilder with proper type binding
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("UPDATE tasks SET ");

        let mut has_updates = false;

        if let Some(name) = &updates.name {
            if name.trim().is_empty() {
                return Err(TaskError::empty_field("name"));
            }
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("name = ");
            query_builder.push_bind(name);
            has_updates = true;
        }

        if let Some(description) = &updates.description {
            if description.trim().is_empty() {
                return Err(TaskError::empty_field("description"));
            }
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("description = ");
            query_builder.push_bind(description);
            has_updates = true;
        }

        if let Some(owner) = &updates.owner_agent_name {
            if owner.trim().is_empty() {
                return Err(TaskError::empty_field("owner_agent_name"));
            }
            if has_updates {
                query_builder.push(", ");
            }
            query_builder.push("owner_agent_name = ");
            query_builder.push_bind(owner);
            has_updates = true;
        }

        if !has_updates {
            // No updates provided, return existing task
            return Ok(existing.unwrap());
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);
        query_builder.push(
            " RETURNING id, code, name, description, owner_agent_name, state, inserted_at, done_at",
        );

        let row = query_builder
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        row_to_task(&row)
    }

    async fn set_state(&self, id: i32, new_state: TaskState) -> Result<Task> {
        // Get current task to validate state transition
        let current_task = self.get_by_id(id).await?;
        let current_task = match current_task {
            Some(task) => task,
            None => return Err(TaskError::not_found_id(id)),
        };

        // Validate state transition
        if !current_task.can_transition_to(new_state) {
            return Err(TaskError::invalid_transition(current_task.state, new_state));
        }

        // Set done_at timestamp when moving to Done state
        let done_at = if new_state == TaskState::Done {
            Some(Utc::now())
        } else {
            None
        };

        let row = sqlx::query(
            "UPDATE tasks SET state = ?, done_at = ? WHERE id = ? RETURNING id, code, name, description, owner_agent_name, state, inserted_at, done_at"
        )
        .bind(state_to_string(new_state))
        .bind(done_at)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        row_to_task(&row)
    }

    async fn get_by_id(&self, id: i32) -> Result<Option<Task>> {
        let result = sqlx::query(
            "SELECT id, code, name, description, owner_agent_name, state, inserted_at, done_at, claimed_at FROM tasks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        match result {
            Some(row) => Ok(Some(row_to_task(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_by_code(&self, code: &str) -> Result<Option<Task>> {
        let result = sqlx::query(
            "SELECT id, code, name, description, owner_agent_name, state, inserted_at, done_at, claimed_at FROM tasks WHERE code = ?"
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        match result {
            Some(row) => Ok(Some(row_to_task(&row)?)),
            None => Ok(None),
        }
    }

    async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>> {
        // Use the modern build_filter_query function with proper QueryBuilder
        use crate::common::build_filter_query;
        #[cfg(debug_assertions)]
        use sqlx::Execute;

        // Debug logging - only in debug builds for performance
        #[cfg(debug_assertions)]
        {
            tracing::debug!("🔍 LIST FILTER DEBUG: filter = {:?}", filter);
        }

        let mut query_builder = build_filter_query(&filter);
        let query = query_builder.build();

        #[cfg(debug_assertions)]
        {
            tracing::debug!("🔍 GENERATED SQL: {}", query.sql());
        }

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        #[cfg(debug_assertions)]
        {
            tracing::debug!("🔍 QUERY RESULT COUNT: {} rows", rows.len());
        }

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row_to_task(&row)?);
        }

        Ok(tasks)
    }

    async fn assign(&self, id: i32, new_owner: &str) -> Result<Task> {
        // Validate new owner name
        if new_owner.trim().is_empty() {
            return Err(TaskError::empty_field("new_owner"));
        }

        // Check if task exists
        if self.get_by_id(id).await?.is_none() {
            return Err(TaskError::not_found_id(id));
        }

        let row = sqlx::query(
            "UPDATE tasks SET owner_agent_name = ? WHERE id = ? RETURNING id, code, name, description, owner_agent_name, state, inserted_at, done_at"
        )
        .bind(new_owner)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        row_to_task(&row)
    }

    async fn archive(&self, id: i32) -> Result<Task> {
        // Get current task to validate it can be archived
        let current_task = self.get_by_id(id).await?;
        let current_task = match current_task {
            Some(task) => task,
            None => return Err(TaskError::not_found_id(id)),
        };

        // Only Done tasks can be archived
        if !current_task.can_transition_to(TaskState::Archived) {
            return Err(TaskError::invalid_transition(
                current_task.state,
                TaskState::Archived,
            ));
        }

        let row = sqlx::query(
            "UPDATE tasks SET state = ? WHERE id = ? RETURNING id, code, name, description, owner_agent_name, state, inserted_at, done_at"
        )
        .bind(state_to_string(TaskState::Archived))
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        row_to_task(&row)
    }

    async fn health_check(&self) -> Result<()> {
        // Simple query to verify database connectivity
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        Ok(())
    }

    async fn get_stats(&self) -> Result<RepositoryStats> {
        // Parallelize all three database queries for better performance
        let (total_result, state_results, owner_results, timestamp_result) = tokio::join!(
            // Get total task count
            sqlx::query("SELECT COUNT(*) as total FROM tasks")
                .fetch_one(&self.pool),

            // Get tasks by state
            sqlx::query("SELECT state, COUNT(*) as count FROM tasks GROUP BY state")
                .fetch_all(&self.pool),

            // Get tasks by owner
            sqlx::query("SELECT owner_agent_name, COUNT(*) as count FROM tasks GROUP BY owner_agent_name")
                .fetch_all(&self.pool),

            // Get latest timestamps
            sqlx::query(
                "SELECT MAX(inserted_at) as latest_created, MAX(done_at) as latest_completed FROM tasks"
            )
            .fetch_one(&self.pool)
        );

        // Handle results and map errors
        let total_result = total_result.map_err(sqlx_error_to_task_error)?;
        let state_results = state_results.map_err(sqlx_error_to_task_error)?;
        let owner_results = owner_results.map_err(sqlx_error_to_task_error)?;
        let timestamp_result = timestamp_result.map_err(sqlx_error_to_task_error)?;

        let total_tasks: i64 = total_result.get("total");

        // Process tasks by state
        let mut tasks_by_state = HashMap::new();
        for row in state_results {
            let state_str: String = row.get("state");
            let state = string_to_state(&state_str)?;
            let count: i64 = row.get("count");
            tasks_by_state.insert(state, count as u64);
        }

        // Process tasks by owner
        let mut tasks_by_owner = HashMap::new();
        for row in owner_results {
            let owner: String = row.get("owner_agent_name");
            let count: i64 = row.get("count");
            tasks_by_owner.insert(owner, count as u64);
        }

        let latest_created: Option<DateTime<Utc>> = timestamp_result.get("latest_created");
        let latest_completed: Option<DateTime<Utc>> = timestamp_result.get("latest_completed");

        Ok(RepositoryStats {
            total_tasks: total_tasks as u64,
            tasks_by_state,
            tasks_by_owner,
            latest_created,
            latest_completed,
        })
    }

    // MCP v2 Advanced Multi-Agent Features

    async fn discover_work(
        &self,
        _agent_name: &str,
        capabilities: &[String],
        max_tasks: u32,
    ) -> Result<Vec<Task>> {
        use crate::common::build_work_discovery_query;

        let mut query_builder = build_work_discovery_query(capabilities, Some(max_tasks as i32));
        let query = query_builder.build();

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        let tasks: Result<Vec<Task>> = rows.iter().map(row_to_task).collect();
        tasks
    }

    async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        // Start transaction for atomic claim with better isolation
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;

        // Use atomic UPDATE with WHERE conditions to prevent race conditions
        // This will only update if the task is in Created state and unowned/owned by same agent
        let now = chrono::Utc::now();
        let updated_rows = sqlx::query(
            r#"
            UPDATE tasks 
            SET owner_agent_name = ?, state = ?, claimed_at = ?
            WHERE id = ? 
              AND state = 'Created' 
              AND (owner_agent_name IS NULL OR owner_agent_name = '' OR owner_agent_name = ?)
            "#,
        )
        .bind(agent_name)
        .bind(crate::common::state_to_string(TaskState::InProgress))
        .bind(now)
        .bind(task_id)
        .bind(agent_name) // Allow re-claiming by same agent
        .execute(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;

        // Check if the update actually affected any rows
        if updated_rows.rows_affected() == 0 {
            // Fetch current state to provide better error message
            let current_task = sqlx::query_as::<_, (String, String)>(
                "SELECT COALESCE(owner_agent_name, ''), state FROM tasks WHERE id = ?",
            )
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;

            if let Some((current_owner, current_state_str)) = current_task {
                let current_state = crate::common::string_to_state(&current_state_str)?;

                // Task exists but couldn't be claimed
                if !current_owner.is_empty() && current_owner != agent_name {
                    return Err(TaskError::AlreadyClaimed(task_id, current_owner));
                } else if current_state != TaskState::Created {
                    return Err(TaskError::invalid_transition(
                        current_state,
                        TaskState::InProgress,
                    ));
                } else {
                    // Should not happen, but handle gracefully
                    return Err(TaskError::Conflict(format!(
                        "Failed to claim task {task_id} due to concurrent modification"
                    )));
                }
            } else {
                // Task doesn't exist
                return Err(TaskError::not_found_id(task_id));
            }
        }

        tx.commit().await.map_err(sqlx_error_to_task_error)?;

        // Return updated task
        self.get_by_id(task_id)
            .await?
            .ok_or_else(|| TaskError::not_found_id(task_id))
    }

    async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task> {
        // Check if agent owns the task
        let current_task =
            sqlx::query_as::<_, (String,)>("SELECT owner_agent_name FROM tasks WHERE id = ?")
                .bind(task_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(sqlx_error_to_task_error)?;

        let (current_owner,) = match current_task {
            Some(task) => task,
            None => return Err(TaskError::not_found_id(task_id)),
        };

        if current_owner != agent_name {
            return Err(TaskError::NotOwned(agent_name.to_string(), task_id));
        }

        // Clear task owner, reset state to Created, and clear claiming timestamp
        sqlx::query("UPDATE tasks SET owner_agent_name = NULL, state = ?, claimed_at = NULL WHERE id = ?")
            .bind(crate::common::state_to_string(TaskState::Created))
            .bind(task_id)
            .execute(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        // Return updated task
        self.get_by_id(task_id)
            .await?
            .ok_or_else(|| TaskError::not_found_id(task_id))
    }

    async fn start_work_session(&self, task_id: i32, agent_name: &str) -> Result<i32> {
        // Verify task exists and agent owns it
        let task = sqlx::query_as::<_, (String,)>(
            "SELECT COALESCE(owner_agent_name, '') FROM tasks WHERE id = ?",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        let (current_owner,) = match task {
            Some(task) => task,
            None => return Err(TaskError::not_found_id(task_id)),
        };

        if current_owner != agent_name {
            return Err(TaskError::NotOwned(agent_name.to_string(), task_id));
        }

        // Create work session record
        let session_id: i32 = sqlx::query_scalar(
            "INSERT INTO work_sessions (task_id, agent_name, started_at) VALUES (?, ?, ?) RETURNING id"
        )
        .bind(task_id)
        .bind(agent_name)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        Ok(session_id)
    }

    async fn end_work_session(
        &self,
        session_id: i32,
        notes: Option<String>,
        productivity_score: Option<f64>,
    ) -> Result<()> {
        // Check if session exists and is still active
        let session_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_sessions WHERE id = ? AND ended_at IS NULL)",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        if !session_exists {
            return Err(TaskError::SessionNotFound(session_id));
        }

        // End the work session with optional notes and productivity score
        sqlx::query(
            "UPDATE work_sessions SET ended_at = ?, notes = ?, productivity_score = ? WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(notes)
        .bind(productivity_score)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        Ok(())
    }

    async fn cleanup_timed_out_tasks(&self, timeout_minutes: i64) -> Result<Vec<Task>> {
        // Calculate timeout threshold
        let timeout_threshold = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes);
        
        // Start transaction for atomic operation
        let mut tx = self.pool.begin().await.map_err(sqlx_error_to_task_error)?;

        // First, find all timed-out tasks
        let timed_out_rows = sqlx::query(
            r#"
            SELECT id, code, name, description, owner_agent_name, state, inserted_at, done_at, claimed_at
            FROM tasks 
            WHERE state = 'InProgress' 
              AND claimed_at IS NOT NULL 
              AND claimed_at < ?
            "#,
        )
        .bind(timeout_threshold)
        .fetch_all(&mut *tx)
        .await
        .map_err(sqlx_error_to_task_error)?;

        // Convert rows to tasks
        let timed_out_tasks: Result<Vec<Task>> = timed_out_rows.iter().map(row_to_task).collect();
        let timed_out_tasks = timed_out_tasks?;

        if !timed_out_tasks.is_empty() {
            // Release all timed-out tasks - reset to Created state and clear owner/claimed_at
            let updated_rows = sqlx::query(
                r#"
                UPDATE tasks 
                SET state = 'Created', owner_agent_name = NULL, claimed_at = NULL
                WHERE state = 'InProgress' 
                  AND claimed_at IS NOT NULL 
                  AND claimed_at < ?
                "#,
            )
            .bind(timeout_threshold)
            .execute(&mut *tx)
            .await
            .map_err(sqlx_error_to_task_error)?;

            // Log the cleanup operation
            tracing::info!(
                "Cleaned up {} timed-out tasks (timeout: {} minutes, rows updated: {})",
                timed_out_tasks.len(),
                timeout_minutes,
                updated_rows.rows_affected()
            );
        }

        tx.commit().await.map_err(sqlx_error_to_task_error)?;

        Ok(timed_out_tasks)
    }
}

#[async_trait]
impl TaskMessageRepository for SqliteTaskRepository {
    async fn create_message(
        &self,
        task_code: &str,
        author_agent_name: &str,
        target_agent_name: Option<&str>,
        message_type: &str,
        content: &str,
        reply_to_message_id: Option<i32>,
    ) -> Result<TaskMessage> {
        // Validate input data
        if task_code.trim().is_empty() {
            return Err(TaskError::empty_field("task_code"));
        }
        if author_agent_name.trim().is_empty() {
            return Err(TaskError::empty_field("author_agent_name"));
        }
        if message_type.trim().is_empty() {
            return Err(TaskError::empty_field("message_type"));
        }
        if content.trim().is_empty() {
            return Err(TaskError::empty_field("content"));
        }

        // Validate that the task exists
        let task_exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tasks WHERE code = ?)")
                .bind(task_code)
                .fetch_one(&self.pool)
                .await
                .map_err(sqlx_error_to_task_error)?;

        if !task_exists {
            return Err(TaskError::not_found_code(task_code));
        }

        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO task_messages (task_code, author_agent_name, target_agent_name, message_type, content, reply_to_message_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING id, task_code, author_agent_name, target_agent_name, message_type, content, reply_to_message_id, created_at
            "#
        )
        .bind(task_code)
        .bind(author_agent_name)
        .bind(target_agent_name)
        .bind(message_type)
        .bind(content)
        .bind(reply_to_message_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        row_to_task_message(&row)
    }

    async fn get_messages(
        &self,
        task_code: &str,
        author_agent_name: Option<&str>,
        target_agent_name: Option<&str>,
        message_type: Option<&str>,
        reply_to_message_id: Option<i32>,
        limit: Option<u32>,
    ) -> Result<Vec<TaskMessage>> {
        // Build dynamic query based on filters
        let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> =
            sqlx::QueryBuilder::new("SELECT id, task_code, author_agent_name, target_agent_name, message_type, content, reply_to_message_id, created_at FROM task_messages WHERE task_code = ");

        query_builder.push_bind(task_code);

        if let Some(author) = author_agent_name {
            query_builder.push(" AND author_agent_name = ");
            query_builder.push_bind(author);
        }

        if let Some(target) = target_agent_name {
            query_builder.push(" AND target_agent_name = ");
            query_builder.push_bind(target);
        }

        if let Some(msg_type) = message_type {
            query_builder.push(" AND message_type = ");
            query_builder.push_bind(msg_type);
        }

        if let Some(reply_id) = reply_to_message_id {
            query_builder.push(" AND reply_to_message_id = ");
            query_builder.push_bind(reply_id);
        }

        query_builder.push(" ORDER BY created_at DESC");

        if let Some(limit) = limit {
            query_builder.push(" LIMIT ");
            query_builder.push_bind(limit);
        }

        let rows = query_builder
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(sqlx_error_to_task_error)?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row_to_task_message(&row)?);
        }

        Ok(messages)
    }

    async fn get_message_by_id(&self, message_id: i32) -> Result<Option<TaskMessage>> {
        let result = sqlx::query(
            "SELECT id, task_code, author_agent_name, target_agent_name, message_type, content, reply_to_message_id, created_at FROM task_messages WHERE id = ?"
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(sqlx_error_to_task_error)?;

        match result {
            Some(row) => Ok(Some(row_to_task_message(&row)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use task_core::models::{NewTask, TaskFilter};

    async fn create_test_repository() -> SqliteTaskRepository {
        // Use a unique timestamp-based name for each test to avoid locking
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = std::thread::current().id();
        let db_name = format!(":memory:test_{timestamp}_{thread_id:?}");
        let repo = SqliteTaskRepository::new(&db_name).await.unwrap();
        repo.migrate().await.unwrap();
        repo
    }

    #[tokio::test]
    async fn test_repository_creation() {
        let repo = create_test_repository().await;
        let result = repo.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_task() {
        let repo = create_test_repository().await;

        let new_task = NewTask::new(
            "TEST-001".to_string(),
            "Test Task".to_string(),
            "A test task for unit testing".to_string(),
            Some("test-agent".to_string()),
        );

        let created_task = repo.create(new_task).await.unwrap();

        assert_eq!(created_task.code, "TEST-001");
        assert_eq!(created_task.name, "Test Task");
        assert_eq!(created_task.state, TaskState::Created);
        assert!(created_task.id > 0);
        assert!(created_task.done_at.is_none());
    }

    #[tokio::test]
    async fn test_claim_task_critical_fix() {
        let repo = create_test_repository().await;

        // Create a test task
        let new_task = NewTask::new(
            "CLAIM-TEST-001".to_string(),
            "Test Claim Fix".to_string(),
            "Testing if claim_task sets state to InProgress".to_string(),
            None, // Start unassigned
        );

        let task = repo.create(new_task).await.unwrap();
        assert_eq!(task.state, TaskState::Created);
        assert!(task.owner_agent_name.is_none());

        // Claim the task
        let claimed_task = repo.claim_task(task.id, "test-agent").await.unwrap();

        // CRITICAL FIX VERIFICATION: Task should now be in InProgress state
        assert_eq!(
            claimed_task.state,
            TaskState::InProgress,
            "CRITICAL BUG: claim_task() must set state to InProgress"
        );
        assert_eq!(
            claimed_task.owner_agent_name,
            Some("test-agent".to_string()),
            "CRITICAL BUG: claim_task() must set owner_agent_name"
        );

        println!("🎉 SUCCESS: claim_task correctly sets state to InProgress!");
    }

    #[tokio::test]
    async fn test_duplicate_code_error() {
        let repo = create_test_repository().await;

        let new_task = NewTask::new(
            "DUPLICATE".to_string(),
            "First Task".to_string(),
            "First task with this code".to_string(),
            Some("test-agent".to_string()),
        );

        // First creation should succeed
        repo.create(new_task.clone()).await.unwrap();

        // Second creation with same code should fail
        let result = repo.create(new_task).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskError::DuplicateCode(_) => {}
            other => panic!("Expected DuplicateCode error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let repo = create_test_repository().await;

        let new_task = NewTask::new(
            "GET-TEST".to_string(),
            "Get Test".to_string(),
            "Test getting tasks by ID".to_string(),
            Some("test-agent".to_string()),
        );

        let created = repo.create(new_task).await.unwrap();
        let retrieved = repo.get_by_id(created.id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, created.id);

        // Test non-existent ID
        let not_found = repo.get_by_id(99999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let repo = create_test_repository().await;

        let new_task = NewTask::new(
            "STATE-TEST".to_string(),
            "State Test".to_string(),
            "Test state transitions".to_string(),
            Some("test-agent".to_string()),
        );

        let mut task = repo.create(new_task).await.unwrap();
        assert_eq!(task.state, TaskState::Created);

        // Valid transition: Created -> InProgress
        task = repo
            .set_state(task.id, TaskState::InProgress)
            .await
            .unwrap();
        assert_eq!(task.state, TaskState::InProgress);

        // Valid transition: InProgress -> Done
        task = repo.set_state(task.id, TaskState::Done).await.unwrap();
        assert_eq!(task.state, TaskState::Done);
        assert!(task.done_at.is_some());

        // Invalid transition: Done -> InProgress
        let result = repo.set_state(task.id, TaskState::InProgress).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            TaskError::InvalidStateTransition(from, to) => {
                assert_eq!(from, TaskState::Done);
                assert_eq!(to, TaskState::InProgress);
            }
            other => panic!("Expected InvalidStateTransition error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_release_task_resets_state() {
        let repo = create_test_repository().await;

        // Create and claim a task
        let new_task = NewTask::new(
            "RELEASE-TEST".to_string(),
            "Release Test".to_string(),
            "Test release task functionality".to_string(),
            None,
        );

        let task = repo.create(new_task).await.unwrap();
        let claimed_task = repo.claim_task(task.id, "test-agent").await.unwrap();

        // Verify task is claimed and in InProgress state
        assert_eq!(claimed_task.state, TaskState::InProgress);
        assert_eq!(
            claimed_task.owner_agent_name,
            Some("test-agent".to_string())
        );

        // Release the task
        let released_task = repo.release_task(task.id, "test-agent").await.unwrap();

        // Verify task is released and back to Created state
        assert_eq!(
            released_task.state,
            TaskState::Created,
            "CRITICAL: release_task() must reset state to Created"
        );
        assert!(
            released_task.owner_agent_name.is_none(),
            "CRITICAL: release_task() must clear owner"
        );

        // Verify task can be claimed again by another agent
        let reclaimed_task = repo.claim_task(task.id, "another-agent").await.unwrap();
        assert_eq!(reclaimed_task.state, TaskState::InProgress);
        assert_eq!(
            reclaimed_task.owner_agent_name,
            Some("another-agent".to_string())
        );

        println!("✅ SUCCESS: release_task correctly resets state to Created!");
    }

    #[tokio::test]
    async fn test_concurrent_claim_task_race_conditions() {
        let repo = create_test_repository().await;

        // Create a task that multiple agents will try to claim
        let new_task = NewTask::new(
            "RACE-TEST".to_string(),
            "Race Condition Test".to_string(),
            "Test concurrent claiming to verify race condition prevention".to_string(),
            None,
        );

        let task = repo.create(new_task).await.unwrap();
        assert_eq!(task.state, TaskState::Created);

        // Create multiple agents trying to claim the same task concurrently
        let agent_names = vec!["agent-1", "agent-2", "agent-3", "agent-4", "agent-5"];
        let mut handles = vec![];

        for agent_name in agent_names {
            let repo_clone = repo.clone();
            let task_id = task.id;
            let agent_name = agent_name.to_string();

            let handle =
                tokio::spawn(async move { repo_clone.claim_task(task_id, &agent_name).await });
            handles.push(handle);
        }

        // Wait for all attempts to complete
        let results: Vec<Result<Task>> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // Exactly one should succeed, others should fail
        let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
        let failures: Vec<_> = results.iter().filter(|r| r.is_err()).collect();

        assert_eq!(
            successes.len(),
            1,
            "Exactly one agent should successfully claim the task"
        );
        assert_eq!(
            failures.len(),
            4,
            "Four agents should fail to claim the task"
        );

        // Verify the successful claim
        let claimed_task = successes[0].as_ref().unwrap();
        assert_eq!(claimed_task.state, TaskState::InProgress);
        assert!(claimed_task.owner_agent_name.is_some());

        // Verify failures are for correct reasons
        for failure in failures {
            match failure.as_ref().unwrap_err() {
                TaskError::AlreadyClaimed(_, _)
                | TaskError::InvalidStateTransition(_, _)
                | TaskError::Conflict(_) => {
                    // These are expected error types for race conditions
                }
                other => panic!("Unexpected error type: {other:?}"),
            }
        }

        println!("✅ SUCCESS: Concurrent claim_task properly handles race conditions!");
    }

    #[tokio::test]
    async fn test_list_with_filters() {
        let repo = create_test_repository().await;

        // Create multiple tasks
        let tasks = vec![
            NewTask::new(
                "AGENT-1-TASK".to_string(),
                "Agent 1 Task".to_string(),
                "Task for agent 1".to_string(),
                Some("agent-1".to_string()),
            ),
            NewTask::new(
                "AGENT-2-TASK".to_string(),
                "Agent 2 Task".to_string(),
                "Task for agent 2".to_string(),
                Some("agent-2".to_string()),
            ),
        ];

        for task in tasks {
            repo.create(task).await.unwrap();
        }

        // Test listing all tasks
        let all_tasks = repo.list(TaskFilter::default()).await.unwrap();
        assert_eq!(all_tasks.len(), 2);

        // Test filtering by owner
        let filter = TaskFilter {
            owner: Some("agent-1".to_string()),
            ..Default::default()
        };
        let agent1_tasks = repo.list(filter).await.unwrap();
        assert_eq!(agent1_tasks.len(), 1);
        assert_eq!(agent1_tasks[0].owner_agent_name.as_deref(), Some("agent-1"));
    }
}
