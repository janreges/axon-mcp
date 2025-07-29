//! MCP Server with SSE Transport
//! 
//! Implements the MCP server using Server-Sent Events for communication.

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Sse,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::sync::mpsc;
use tracing::info;

use crate::{error::McpError, handler::McpTaskHandler, serialization::*};
use ::task_core::{TaskRepository, ProtocolHandler};

/// MCP Server with SSE transport
pub struct McpServer<R> {
    handler: McpTaskHandler<R>,
}

impl<R: TaskRepository + Send + Sync + 'static> McpServer<R> {
    /// Create new MCP server
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            handler: McpTaskHandler::new(repository),
        }
    }
    
    /// Start the MCP server
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();
        
        info!("Starting MCP server on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    /// Create the router with all endpoints
    fn create_router(self) -> Router {
        Router::new()
            .route("/mcp/v1", get(sse_handler))
            .route("/mcp/v1/rpc", post(rpc_handler))
            .route("/health", get(health_handler))
            .with_state(Arc::new(self.handler))
    }
    
    /// Route MCP method to appropriate handler - shared logic for both SSE and RPC
    async fn route_method(&self, method: &str, params: Value, id: Option<Value>) -> Value {
        let result = self.execute_method(method, params).await;
        
        match result {
            Ok(value) => create_success_response(id, value),
            Err(err) => err.to_json_rpc_error(id),
        }
    }
    
    /// Execute MCP method - core method routing logic
    async fn execute_method(&self, method: &str, params: Value) -> Result<Value, McpError> {
        match method {
            "create_task" => {
                let params: CreateTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.create_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "update_task" => {
                let params: UpdateTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.update_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "set_task_state" => {
                let params: SetStateParams = deserialize_mcp_params(params)?;
                let task = self.handler.set_task_state(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "get_task_by_id" => {
                let params: GetTaskByIdParams = deserialize_mcp_params(params)?;
                match self.handler.get_task_by_id(params).await.map_err(McpError::from)? {
                    Some(task) => serialize_task_for_mcp(&task),
                    None => Ok(Value::Null),
                }
            }
            "get_task_by_code" => {
                let params: GetTaskByCodeParams = deserialize_mcp_params(params)?;
                match self.handler.get_task_by_code(params).await.map_err(McpError::from)? {
                    Some(task) => serialize_task_for_mcp(&task),
                    None => Ok(Value::Null),
                }
            }
            "list_tasks" => {
                let params: ListTasksParams = deserialize_mcp_params(params)?;
                let tasks = self.handler.list_tasks(params).await.map_err(McpError::from)?;
                let task_values: Result<Vec<_>, _> = tasks.iter()
                    .map(serialize_task_for_mcp)
                    .collect();
                Ok(Value::Array(task_values?))
            }
            "assign_task" => {
                let params: AssignTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.assign_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "archive_task" => {
                let params: ArchiveTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.archive_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "health_check" => {
                let health = self.handler.health_check().await.map_err(McpError::from)?;
                Ok(serde_json::to_value(health).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            _ => Err(McpError::Protocol(format!("Unknown method: {method}"))),
        }
    }
}

/// SSE endpoint for MCP communication
async fn sse_handler<R: TaskRepository + Send + Sync + 'static>(
    State(_handler): State<Arc<McpTaskHandler<R>>>,
) -> Result<Sse<UnboundedReceiverStream<Result<axum::response::sse::Event, axum::Error>>>, StatusCode> {
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Send initial connection event
    let welcome_event = axum::response::sse::Event::default()
        .data(json!({
            "jsonrpc": "2.0",
            "method": "connection_established",
            "params": {
                "server": "mcp-task-server",
                "version": env!("CARGO_PKG_VERSION"),
                "capabilities": [
                    "create_task", "update_task", "set_task_state",
                    "get_task_by_id", "get_task_by_code", "list_tasks",
                    "assign_task", "archive_task"
                ]
            }
        }).to_string());
    
    if tx.send(Ok(welcome_event)).is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    
    // Set up heartbeat
    let heartbeat_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let heartbeat = axum::response::sse::Event::default()
                .event("heartbeat")
                .data("ping");
            
            if heartbeat_tx.send(Ok(heartbeat)).is_err() {
                break;
            }
        }
    });
    
    let stream = UnboundedReceiverStream::new(rx);
    Ok(Sse::new(stream))
}

/// JSON-RPC endpoint for MCP communication
async fn rpc_handler<R: TaskRepository + Send + Sync + 'static>(
    State(handler): State<Arc<McpTaskHandler<R>>>,
    Json(request): Json<Value>,
) -> Json<Value> {
    info!("Received RPC request: {}", request);
    
    // Extract ID first for error responses
    let id = request.get("id").cloned();
    
    // Parse JSON-RPC request - return JSON-RPC errors instead of HTTP errors
    let method = match request.get("method").and_then(|v| v.as_str()) {
        Some(method) => method,
        None => {
            let error = McpError::Protocol("Missing or invalid 'method' field in JSON-RPC request".to_string());
            return Json(error.to_json_rpc_error(id));
        }
    };
    
    let params = request.get("params").unwrap_or(&Value::Null).clone();
    
    // Create temporary server instance to use shared routing logic
    let server = McpServer { handler: McpTaskHandler::new(handler.repository()) };
    let response = server.route_method(method, params, id).await;
    
    Json(response)
}

/// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::mock;
    use async_trait::async_trait;
    use ::task_core::{Task, NewTask, UpdateTask, TaskFilter, TaskState, RepositoryStats};
    use ::task_core::error::Result;

    mock! {
        TestRepository {}
        
        #[async_trait]
        impl TaskRepository for TestRepository {
            async fn create(&self, task: NewTask) -> Result<Task>;
            async fn update(&self, id: i32, updates: UpdateTask) -> Result<Task>;
            async fn set_state(&self, id: i32, state: TaskState) -> Result<Task>;
            async fn get_by_id(&self, id: i32) -> Result<Option<Task>>;
            async fn get_by_code(&self, code: &str) -> Result<Option<Task>>;
            async fn list(&self, filter: TaskFilter) -> Result<Vec<Task>>;
            async fn assign(&self, id: i32, new_owner: &str) -> Result<Task>;
            async fn archive(&self, id: i32) -> Result<Task>;
            async fn health_check(&self) -> Result<()>;
            async fn get_stats(&self) -> Result<RepositoryStats>;
        }
    }
    
    #[test]
    fn test_server_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let _server = McpServer::new(mock_repo);
        // Basic test that server can be created
        assert!(true);
    }
}