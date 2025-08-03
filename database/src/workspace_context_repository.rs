use async_trait::async_trait;
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use task_core::{
    error::{Result, TaskError},
    workspace_setup::WorkspaceContext,
    WorkspaceContextRepository,
};

/// SQLite implementation of WorkspaceContextRepository trait
///
/// This implementation stores entire WorkspaceContext as JSON in SQLite
/// to minimize schema complexity and fully utilize existing serde implementation.
#[derive(Clone)]
pub struct SqliteWorkspaceContextRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteWorkspaceContextRepository {
    /// Create a new SQLite workspace context repository
    ///
    /// # Arguments
    /// * `pool` - The SQLite connection pool
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkspaceContextRepository for SqliteWorkspaceContextRepository {
    async fn create(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
        let now = Utc::now();
        let serialized_data = serde_json::to_string(&context).map_err(|e| {
            TaskError::Serialization(format!("Failed to serialize WorkspaceContext: {e}"))
        })?;

        let result = sqlx::query(
            "INSERT INTO workspace_contexts (workspace_id, data, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&context.workspace_id)
        .bind(&serialized_data)
        .bind(context.version)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&*self.pool)
        .await;

        match result {
            Ok(_) => Ok(context),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(TaskError::DuplicateKey(format!(
                    "Workspace ID '{}' already exists",
                    context.workspace_id
                )))
            }
            Err(e) => Err(TaskError::Database(format!(
                "Failed to create workspace context: {e}"
            ))),
        }
    }

    async fn get_by_id(&self, workspace_id: &str) -> Result<Option<WorkspaceContext>> {
        let row = sqlx::query("SELECT data FROM workspace_contexts WHERE workspace_id = ?")
            .bind(workspace_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| TaskError::Database(format!("Failed to get workspace context: {e}")))?;

        match row {
            Some(row) => {
                let data_str: String = row.get("data");
                let context: WorkspaceContext = serde_json::from_str(&data_str).map_err(|e| {
                    TaskError::Deserialization(format!(
                        "Failed to deserialize WorkspaceContext: {e}"
                    ))
                })?;
                Ok(Some(context))
            }
            None => Ok(None),
        }
    }

    async fn update(&self, mut context: WorkspaceContext) -> Result<WorkspaceContext> {
        let now = Utc::now();
        let old_version = context.version;
        context.version += 1; // Increment version for optimistic locking
        context.updated_at = now;

        let serialized_data = serde_json::to_string(&context).map_err(|e| {
            TaskError::Serialization(format!("Failed to serialize WorkspaceContext: {e}"))
        })?;

        // Optimistic locking: only update if version matches
        let result = sqlx::query(
            "UPDATE workspace_contexts SET data = ?, version = ?, updated_at = ? WHERE workspace_id = ? AND version = ?"
        )
        .bind(&serialized_data)
        .bind(context.version)
        .bind(now.to_rfc3339())
        .bind(&context.workspace_id)
        .bind(old_version) // Check old version for optimistic locking
        .execute(&*self.pool)
        .await
        .map_err(|e| TaskError::Database(format!("Failed to update workspace context: {e}")))?;

        if result.rows_affected() == 0 {
            // Check if workspace exists at all
            let exists = sqlx::query("SELECT 1 FROM workspace_contexts WHERE workspace_id = ?")
                .bind(&context.workspace_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| {
                    TaskError::Database(format!("Failed to check workspace existence: {e}"))
                })?;

            if exists.is_none() {
                return Err(TaskError::NotFound(format!(
                    "Workspace ID '{}' not found",
                    context.workspace_id
                )));
            } else {
                return Err(TaskError::Conflict(format!(
                    "Workspace '{}' was modified by another operation (version conflict)",
                    context.workspace_id
                )));
            }
        }

        Ok(context)
    }

    async fn delete(&self, workspace_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM workspace_contexts WHERE workspace_id = ?")
            .bind(workspace_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| TaskError::Database(format!("Failed to delete workspace context: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(TaskError::NotFound(format!(
                "Workspace ID '{workspace_id}' not found"
            )));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| TaskError::Database(format!("Health check failed: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use task_core::workspace_setup::WorkspaceContext;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create the workspace_contexts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workspace_contexts (
                workspace_id TEXT PRIMARY KEY NOT NULL,
                data TEXT NOT NULL,
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_and_get_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("test-workspace-1".to_string());

        // Create context
        let created = repo.create(context.clone()).await.unwrap();
        assert_eq!(created.workspace_id, "test-workspace-1");

        // Get context
        let retrieved = repo.get_by_id("test-workspace-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().workspace_id, "test-workspace-1");
    }

    #[tokio::test]
    async fn test_create_duplicate_workspace_id() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("duplicate-workspace".to_string());

        // Create first context
        repo.create(context.clone()).await.unwrap();

        // Try to create duplicate
        let result = repo.create(context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskError::DuplicateKey(_)));
    }

    #[tokio::test]
    async fn test_update_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("update-test".to_string());

        // Create context, initial version is 1
        let created_context = repo.create(context.clone()).await.unwrap();
        assert_eq!(created_context.version, 1);

        // Update the context. The update function will handle the version increment.
        let updated = repo.update(created_context).await.unwrap();
        assert_eq!(updated.version, 2); // Version should now be 2

        // Verify update
        let retrieved = repo.get_by_id("update-test").await.unwrap().unwrap();
        assert_eq!(retrieved.version, 2);
    }

    #[tokio::test]
    async fn test_update_conflict() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("conflict-test".to_string());
        repo.create(context.clone()).await.unwrap();

        // 1. Fetch the context twice, simulating two concurrent processes
        let mut context1 = repo.get_by_id("conflict-test").await.unwrap().unwrap();
        let mut context2 = repo.get_by_id("conflict-test").await.unwrap().unwrap();

        // 2. First process updates the context successfully
        context1.prd_content = Some("update 1".to_string());
        let updated1 = repo.update(context1).await.unwrap();
        assert_eq!(updated1.version, 2);

        // 3. Second process tries to update using stale data
        context2.prd_content = Some("update 2".to_string());
        let result2 = repo.update(context2).await;

        // 4. Assert that the second update failed with a conflict error
        assert!(result2.is_err());
        assert!(matches!(result2.unwrap_err(), TaskError::Conflict(_)));

        // 5. Verify that the first update is still the one in the DB
        let final_context = repo.get_by_id("conflict-test").await.unwrap().unwrap();
        assert_eq!(final_context.version, 2);
        assert_eq!(final_context.prd_content.unwrap(), "update 1");
    }

    #[tokio::test]
    async fn test_delete_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("delete-test".to_string());

        // Create context
        repo.create(context.clone()).await.unwrap();

        // Delete context
        repo.delete("delete-test").await.unwrap();

        // Verify deletion
        let retrieved = repo.get_by_id("delete-test").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let retrieved = repo.get_by_id("nonexistent").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update_nonexistent_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let context = WorkspaceContext::new("nonexistent".to_string());

        let result = repo.update(context).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_workspace_context() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        let result = repo.delete("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TaskError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = setup_test_db().await;
        let repo = SqliteWorkspaceContextRepository::new(Arc::new(pool));

        repo.health_check().await.unwrap();
    }
}
