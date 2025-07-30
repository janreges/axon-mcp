//! MCP v2 Server with Multiple Transport Support
//! 
//! Implements the MCP server supporting both modern Streamable HTTP transport (2025-06-18)
//! and legacy Server-Sent Events for backward compatibility.

use std::sync::Arc;
use axum::{
    extract::State,
    http::{StatusCode, HeaderMap, header},
    response::Sse,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::sync::mpsc;
use tracing::info;

use crate::{auth::McpAuth, error::McpError, handler::McpTaskHandler, serialization::*};
use ::task_core::{TaskRepository, ProtocolHandler};

/// MCP Protocol Version as required by 2025-06-18 specification
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Shared server state for handlers
#[derive(Clone)]
pub struct McpServerState<R> {
    pub handler: McpTaskHandler<R>,
    pub auth: McpAuth,
}

/// MCP Server with multiple transport support
pub struct McpServer<R> {
    handler: McpTaskHandler<R>,
    auth: McpAuth,
}

impl<R: TaskRepository + Send + Sync + 'static> McpServer<R> {
    /// Create new MCP server with authentication disabled (development mode)
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            handler: McpTaskHandler::new(repository),
            auth: McpAuth::new(false), // Disabled by default for backward compatibility
        }
    }
    
    /// Create new MCP server with authentication enabled (production mode)
    pub fn new_with_auth(repository: Arc<R>, auth_enabled: bool) -> Self {
        Self {
            handler: McpTaskHandler::new(repository),
            auth: McpAuth::new(auth_enabled),
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
        let state = Arc::new(McpServerState {
            handler: self.handler,
            auth: self.auth,
        });
        
        Router::new()
            .route("/mcp", post(rpc_handler)) // MCP 2025-06-18 Streamable HTTP transport
            .route("/mcp/v1", get(sse_handler)) // Legacy SSE support (deprecated)
            .route("/mcp/v1/rpc", post(rpc_handler)) // Legacy RPC support (deprecated)  
            .route("/health", get(health_handler))
            .with_state(state)
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

/// Execute MCP method - shared logic for both server instances and handlers
async fn execute_mcp_method<R: TaskRepository + Send + Sync>(
    handler: &McpTaskHandler<R>, 
    method: &str, 
    params: Value, 
    id: Option<Value>
) -> Value {
    match method {
        "create_task" => {
            let params: CreateTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.create_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "update_task" => {
            let params: UpdateTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.update_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "set_task_state" => {
            let params: SetStateParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.set_task_state(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_task_by_id" => {
            let params: GetTaskByIdParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_task_by_id(params).await {
                Ok(Some(task)) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Ok(None) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_task_by_code" => {
            let params: GetTaskByCodeParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_task_by_code(params).await {
                Ok(Some(task)) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Ok(None) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "list_tasks" => {
            let params: ListTasksParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.list_tasks(params).await {
                Ok(tasks) => {
                    let task_values: Result<Vec<_>, _> = tasks.iter()
                        .map(serialize_task_for_mcp)
                        .collect();
                    match task_values {
                        Ok(values) => create_success_response(id, Value::Array(values)),
                        Err(e) => McpError::from(e).to_json_rpc_error(id),
                    }
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "assign_task" => {
            let params: AssignTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.assign_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "archive_task" => {
            let params: ArchiveTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.archive_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "health_check" => {
            match handler.health_check().await {
                Ok(health) => match serde_json::to_value(health) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        _ => {
            McpError::Protocol(format!("Unknown method: {method}")).to_json_rpc_error(id)
        }
    }
}

/// SSE endpoint for MCP communication
async fn sse_handler<R: TaskRepository + Send + Sync + 'static>(
    State(_state): State<Arc<McpServerState<R>>>,
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
                "protocol_version": MCP_PROTOCOL_VERSION,
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
    State(state): State<Arc<McpServerState<R>>>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<(HeaderMap, Json<Value>), StatusCode> {
    info!("Received RPC request: {}", request);
    
    // Extract ID first for error responses
    let id = request.get("id").cloned();
    
    // Validate MCP-Protocol-Version header (required by 2025-06-18 spec)
    let protocol_version = headers.get("MCP-Protocol-Version")
        .or_else(|| headers.get("mcp-protocol-version")) // Try lowercase variant
        .and_then(|v| v.to_str().ok());
    
    // Set response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::HeaderName::from_static("mcp-protocol-version"),
        MCP_PROTOCOL_VERSION.parse().unwrap()
    );
    response_headers.insert(
        header::CONTENT_TYPE,
        "application/json".parse().unwrap()
    );
    
    // Validate protocol version - default to 2025-03-26 for backward compatibility if missing
    match protocol_version {
        Some(version) if version == MCP_PROTOCOL_VERSION => {
            // Current version - proceed normally
        }
        Some(version) if version == "2025-03-26" => {
            // Backward compatibility - log warning but proceed
            info!("Client using older MCP protocol version: {}", version);
        }
        Some(version) => {
            // Unsupported version
            let error = McpError::Protocol(format!("Unsupported MCP-Protocol-Version: {}. Supported versions: {}, 2025-03-26", version, MCP_PROTOCOL_VERSION));
            return Ok((response_headers, Json(error.to_json_rpc_error(id))));
        }
        None => {
            // Missing header - default to backward compatibility
            info!("Missing MCP-Protocol-Version header, defaulting to 2025-03-26 compatibility mode");
        }
    }
    
    // Validate that request is not a JSON-RPC batch (forbidden in 2025-06-18)
    if request.is_array() {
        let error = McpError::Protocol("JSON-RPC batching is not supported in MCP 2025-06-18 specification".to_string());
        return Ok((response_headers, Json(error.to_json_rpc_error(id))));
    }
    
    // Validate authentication for the request
    let token_validation = state.auth.validate_token(&headers).await;
    if !token_validation.is_valid {
        let auth_error = McpAuth::create_auth_error("invalid_token", "The provided authentication token is invalid", id);
        return Ok((response_headers, auth_error));
    }
    
    // Parse JSON-RPC request - return JSON-RPC errors instead of HTTP errors
    let method = match request.get("method").and_then(|v| v.as_str()) {
        Some(method) => method,
        None => {
            let error = McpError::Protocol("Missing or invalid 'method' field in JSON-RPC request".to_string());
            return Ok((response_headers, Json(error.to_json_rpc_error(id))));
        }
    };
    
    let params = request.get("params").unwrap_or(&Value::Null).clone();
    
    // Validate that the token has the required scope for this method
    if !state.auth.check_scope(&token_validation, method) {
        let auth_error = McpAuth::create_auth_error("insufficient_scope", &format!("Token does not have required scope for method: {}", method), id);
        return Ok((response_headers, auth_error));
    }
    
    // Execute the method directly through the handler
    let response = execute_mcp_method(&state.handler, method, params, id).await;
    
    Ok((response_headers, Json(response)))
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