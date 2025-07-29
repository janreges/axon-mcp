use task_core::{
    error::{Result, TaskError},
    models::{Task, TaskState, TaskFilter},
};
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqliteRow};

/// Convert TaskState enum to string for database storage
pub fn state_to_string(state: TaskState) -> &'static str {
    match state {
        TaskState::Created => "Created",
        TaskState::InProgress => "InProgress",
        TaskState::Blocked => "Blocked",
        TaskState::Review => "Review",
        TaskState::Done => "Done",
        TaskState::Archived => "Archived",
    }
}

/// Convert string from database to TaskState enum
pub fn string_to_state(s: &str) -> Result<TaskState> {
    match s {
        "Created" => Ok(TaskState::Created),
        "InProgress" => Ok(TaskState::InProgress),
        "Blocked" => Ok(TaskState::Blocked),
        "Review" => Ok(TaskState::Review),
        "Done" => Ok(TaskState::Done),
        "Archived" => Ok(TaskState::Archived),
        _ => Err(TaskError::Database(format!("Invalid task state in database: {s}"))),
    }
}

/// Convert SQLite row to Task model
pub fn row_to_task(row: &SqliteRow) -> Result<Task> {
    let state_str: String = row.get("state");
    let state = string_to_state(&state_str)?;
    
    let inserted_at: DateTime<Utc> = row.get("inserted_at");
    let done_at: Option<DateTime<Utc>> = row.get("done_at");
    
    Ok(Task {
        id: row.get("id"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        owner_agent_name: row.get("owner_agent_name"),
        state,
        inserted_at,
        done_at,
    })
}

/// Convert SQLx error to TaskError
pub fn sqlx_error_to_task_error(err: sqlx::Error) -> TaskError {
    match &err {
        sqlx::Error::Database(db_err) => {
            let code = db_err.code().unwrap_or_default();
            let message = db_err.message();
            
            // Handle SQLite constraint violations
            if code == "2067" || message.contains("UNIQUE constraint failed") {
                // Extract the constraint name to determine which field failed
                if message.contains("tasks.code") {
                    let parts: Vec<&str> = message.split('.').collect();
                    if let Some(last_part) = parts.last() {
                        let code_value = last_part.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
                        return TaskError::DuplicateCode(code_value.to_string());
                    }
                }
                TaskError::DuplicateCode("unknown".to_string())
            } else {
                TaskError::Database(format!("Database constraint error: {message}"))
            }
        },
        sqlx::Error::RowNotFound => {
            // This is handled at the application level, not an error
            TaskError::Database("Unexpected RowNotFound error".to_string())
        },
        sqlx::Error::PoolTimedOut => {
            TaskError::Database("Connection pool timeout".to_string())
        },
        sqlx::Error::Io(io_err) => {
            TaskError::Database(format!("Database I/O error: {io_err}"))
        },
        _ => {
            TaskError::Database(format!("Database operation failed: {err}"))
        }
    }
}

/// Build dynamic WHERE clause for task filtering using QueryBuilder with proper type binding
#[allow(dead_code)] // Used in sqlite.rs but may not be detected by compiler
pub fn build_filter_query(filter: &TaskFilter) -> sqlx::QueryBuilder<sqlx::Sqlite> {
    let mut query_builder: sqlx::QueryBuilder<sqlx::Sqlite> = 
        sqlx::QueryBuilder::new("SELECT id, code, name, description, owner_agent_name, state, inserted_at, done_at FROM tasks");
    
    let mut has_conditions = false;
    
    if let Some(ref owner) = filter.owner {
        query_builder.push(" WHERE owner_agent_name = ");
        query_builder.push_bind(owner);
        has_conditions = true;
    }
    
    if let Some(state) = filter.state {
        if has_conditions {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
            has_conditions = true;
        }
        query_builder.push("state = ");
        query_builder.push_bind(state_to_string(state));
    }
    
    if let Some(date_from) = filter.date_from {
        if has_conditions {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
            has_conditions = true;
        }
        query_builder.push("inserted_at >= ");
        query_builder.push_bind(date_from.to_rfc3339());
    }
    
    if let Some(date_to) = filter.date_to {
        if has_conditions {
            query_builder.push(" AND ");
        } else {
            query_builder.push(" WHERE ");
        }
        query_builder.push("inserted_at <= ");
        query_builder.push_bind(date_to.to_rfc3339());
    }
    
    query_builder.push(" ORDER BY inserted_at DESC");
    
    if let Some(limit) = filter.limit {
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
    }
    
    if let Some(offset) = filter.offset {
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);
    }
    
    query_builder
}

/// Legacy function kept for backward compatibility with tests
/// Build dynamic WHERE clause for task filtering (returns string and params)
/// 
/// DEPRECATED: Use build_filter_query instead for proper type binding
#[allow(dead_code)]
pub fn build_filter_conditions(filter: &TaskFilter) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut params = Vec::new();
    
    if let Some(ref owner) = filter.owner {
        conditions.push("owner_agent_name = ?".to_string());
        params.push(owner.clone());
    }
    
    if let Some(state) = filter.state {
        conditions.push("state = ?".to_string());
        params.push(state_to_string(state).to_string());
    }
    
    if let Some(date_from) = filter.date_from {
        conditions.push("inserted_at >= ?".to_string());
        params.push(date_from.to_rfc3339());
    }
    
    if let Some(date_to) = filter.date_to {
        conditions.push("inserted_at <= ?".to_string());
        params.push(date_to.to_rfc3339());
    }
    
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    
    (where_clause, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_state_conversions() {
        // Test all state conversions
        assert_eq!(state_to_string(TaskState::Created), "Created");
        assert_eq!(state_to_string(TaskState::InProgress), "InProgress");
        assert_eq!(state_to_string(TaskState::Blocked), "Blocked");
        assert_eq!(state_to_string(TaskState::Review), "Review");
        assert_eq!(state_to_string(TaskState::Done), "Done");
        assert_eq!(state_to_string(TaskState::Archived), "Archived");

        assert_eq!(string_to_state("Created").unwrap(), TaskState::Created);
        assert_eq!(string_to_state("InProgress").unwrap(), TaskState::InProgress);
        assert_eq!(string_to_state("Blocked").unwrap(), TaskState::Blocked);
        assert_eq!(string_to_state("Review").unwrap(), TaskState::Review);
        assert_eq!(string_to_state("Done").unwrap(), TaskState::Done);
        assert_eq!(string_to_state("Archived").unwrap(), TaskState::Archived);

        // Test invalid state
        assert!(string_to_state("Invalid").is_err());
    }

    #[test]
    fn test_filter_conditions() {
        // Test empty filter
        let filter = TaskFilter::default();
        let (where_clause, params) = build_filter_conditions(&filter);
        assert_eq!(where_clause, "");
        assert!(params.is_empty());

        // Test filter with owner
        let filter = TaskFilter {
            owner: Some("test-agent".to_string()),
            ..Default::default()
        };
        let (where_clause, params) = build_filter_conditions(&filter);
        assert_eq!(where_clause, "WHERE owner_agent_name = ?");
        assert_eq!(params, vec!["test-agent"]);

        // Test filter with state
        let filter = TaskFilter {
            state: Some(TaskState::InProgress),
            ..Default::default()
        };
        let (where_clause, params) = build_filter_conditions(&filter);
        assert_eq!(where_clause, "WHERE state = ?");
        assert_eq!(params, vec!["InProgress"]);

        // Test filter with multiple conditions
        let filter = TaskFilter {
            owner: Some("test-agent".to_string()),
            state: Some(TaskState::Done),
            date_from: Some(Utc::now()),
            date_to: Some(Utc::now()),
            limit: None,
            offset: None,
        };
        let (where_clause, params) = build_filter_conditions(&filter);
        assert!(where_clause.starts_with("WHERE"));
        assert!(where_clause.contains("owner_agent_name = ?"));
        assert!(where_clause.contains("state = ?"));
        assert!(where_clause.contains("inserted_at >= ?"));
        assert!(where_clause.contains("inserted_at <= ?"));
        assert_eq!(params.len(), 4);
    }

    #[test]
    fn test_proper_type_binding() {
        use chrono::Utc;
        use sqlx::Execute;
        
        // Test that the new query builder properly handles different types
        let filter = TaskFilter {
            owner: Some("test-agent".to_string()),
            state: Some(TaskState::InProgress),
            date_from: Some(Utc::now()),
            date_to: Some(Utc::now()),
            limit: Some(10),
            offset: Some(5),
        };
        
        // This should not panic or cause type errors when building
        let mut query_builder = build_filter_query(&filter);
        let query = query_builder.build();
        
        // The query should contain the expected SQL structure
        let sql = query.sql();
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("WHERE owner_agent_name = "));
        assert!(sql.contains("AND state = "));
        assert!(sql.contains("AND inserted_at >= "));
        assert!(sql.contains("AND inserted_at <= "));
        assert!(sql.contains("ORDER BY inserted_at DESC"));
        assert!(sql.contains("LIMIT "));
        assert!(sql.contains("OFFSET "));
    }
}