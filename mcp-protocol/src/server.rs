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
use ::task_core::{TaskRepository, TaskMessageRepository, ProtocolHandler, DiscoverWorkParams, ClaimTaskParams, ReleaseTaskParams, StartWorkSessionParams, EndWorkSessionParams, CreateTaskMessageParams, GetTaskMessagesParams};

/// MCP Protocol Version as required by 2025-06-18 specification
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Shared server state for handlers
#[derive(Clone)]
pub struct McpServerState<R, M> {
    pub handler: McpTaskHandler<R, M>,
    pub auth: McpAuth,
}

/// MCP Server with multiple transport support
pub struct McpServer<R, M> {
    handler: McpTaskHandler<R, M>,
    auth: McpAuth,
}

impl<R: TaskRepository + Send + Sync + 'static, M: TaskMessageRepository + Send + Sync + 'static> McpServer<R, M> {
    /// Create new MCP server with authentication disabled (development mode)
    pub fn new(repository: Arc<R>, message_repository: Arc<M>) -> Self {
        Self {
            handler: McpTaskHandler::new(repository, message_repository),
            auth: McpAuth::new(false), // Disabled by default for backward compatibility
        }
    }
    
    /// Create new MCP server with authentication enabled (production mode)
    pub fn new_with_auth(repository: Arc<R>, message_repository: Arc<M>, auth_enabled: bool) -> Self {
        Self {
            handler: McpTaskHandler::new(repository, message_repository),
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
    #[allow(dead_code)]
    async fn route_method(&self, method: &str, params: Value, id: Option<Value>) -> Value {
        let result = self.execute_method(method, params).await;
        
        match result {
            Ok(value) => create_success_response(id, value),
            Err(err) => err.to_json_rpc_error(id),
        }
    }
    
    /// Execute MCP method - core method routing logic
    #[allow(dead_code)]
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
            // MCP v2 Advanced Multi-Agent Functions
            "discover_work" => {
                let params: DiscoverWorkParams = deserialize_mcp_params(params)?;
                let tasks = self.handler.discover_work(params).await.map_err(McpError::from)?;
                let task_values: Result<Vec<_>, _> = tasks.iter()
                    .map(serialize_task_for_mcp)
                    .collect();
                Ok(Value::Array(task_values?))
            }
            "claim_task" => {
                let params: ClaimTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.claim_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "release_task" => {
                let params: ReleaseTaskParams = deserialize_mcp_params(params)?;
                let task = self.handler.release_task(params).await.map_err(McpError::from)?;
                serialize_task_for_mcp(&task)
            }
            "start_work_session" => {
                let params: StartWorkSessionParams = deserialize_mcp_params(params)?;
                let session_info = self.handler.start_work_session(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(session_info).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "end_work_session" => {
                let params: EndWorkSessionParams = deserialize_mcp_params(params)?;
                self.handler.end_work_session(params).await.map_err(McpError::from)?;
                Ok(Value::Null) // Success with no return value
            }
            // Task Messaging Functions
            "create_task_message" => {
                let params: CreateTaskMessageParams = deserialize_mcp_params(params)?;
                let message = self.handler.create_task_message(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(message).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "get_task_messages" => {
                let params: GetTaskMessagesParams = deserialize_mcp_params(params)?;
                let messages = self.handler.get_task_messages(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(messages).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            // Workspace Setup Functions
            "get_setup_instructions" => {
                let params: ::task_core::GetSetupInstructionsParams = deserialize_mcp_params(params)?;
                let instructions = self.handler.get_setup_instructions(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(instructions).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "get_agentic_workflow_description" => {
                let params: ::task_core::GetAgenticWorkflowDescriptionParams = deserialize_mcp_params(params)?;
                let workflow = self.handler.get_agentic_workflow_description(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(workflow).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "register_agent" => {
                let params: ::task_core::RegisterAgentParams = deserialize_mcp_params(params)?;
                let agent = self.handler.register_agent(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(agent).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "get_instructions_for_main_ai_file" => {
                let params: ::task_core::GetInstructionsForMainAiFileParams = deserialize_mcp_params(params)?;
                let instructions = self.handler.get_instructions_for_main_ai_file(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(instructions).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "create_main_ai_file" => {
                let params: ::task_core::CreateMainAiFileParams = deserialize_mcp_params(params)?;
                let file_data = self.handler.create_main_ai_file(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(file_data).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            "get_workspace_manifest" => {
                let params: ::task_core::GetWorkspaceManifestParams = deserialize_mcp_params(params)?;
                let manifest = self.handler.get_workspace_manifest(params).await.map_err(McpError::from)?;
                Ok(serde_json::to_value(manifest).map_err(|e| McpError::Serialization(e.to_string()))?)
            }
            _ => Err(McpError::Protocol(format!("Unknown method: {method}"))),
        }
    }
}

/// Execute MCP method - shared logic for both server instances and handlers
async fn execute_mcp_method<R: TaskRepository + Send + Sync, M: TaskMessageRepository + Send + Sync>(
    handler: &McpTaskHandler<R, M>, 
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
        // MCP v2 Advanced Multi-Agent Functions
        "discover_work" => {
            let params: DiscoverWorkParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.discover_work(params).await {
                Ok(tasks) => {
                    let task_values: Result<Vec<_>, _> = tasks.iter()
                        .map(serialize_task_for_mcp)
                        .collect();
                    match task_values {
                        Ok(values) => create_success_response(id, Value::Array(values)),
                        Err(e) => McpError::from(e).to_json_rpc_error(id),
                    }
                }
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "claim_task" => {
            let params: ClaimTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.claim_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "release_task" => {
            let params: ReleaseTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.release_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::from(e).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "start_work_session" => {
            let params: StartWorkSessionParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.start_work_session(params).await {
                Ok(session_info) => {
                    match serde_json::to_value(session_info) {
                        Ok(value) => create_success_response(id, value),
                        Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                    }
                }
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "end_work_session" => {
            let params: EndWorkSessionParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.end_work_session(params).await {
                Ok(()) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        // Task Messaging Functions
        "create_task_message" => {
            let params: CreateTaskMessageParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.create_task_message(params).await {
                Ok(message) => match serde_json::to_value(message) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_task_messages" => {
            let params: GetTaskMessagesParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_task_messages(params).await {
                Ok(messages) => match serde_json::to_value(messages) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        // Workspace Setup Functions
        "get_setup_instructions" => {
            let params: ::task_core::GetSetupInstructionsParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_setup_instructions(params).await {
                Ok(instructions) => match serde_json::to_value(instructions) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_agentic_workflow_description" => {
            let params: ::task_core::GetAgenticWorkflowDescriptionParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_agentic_workflow_description(params).await {
                Ok(workflow) => match serde_json::to_value(workflow) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "register_agent" => {
            let params: ::task_core::RegisterAgentParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.register_agent(params).await {
                Ok(agent) => match serde_json::to_value(agent) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_instructions_for_main_ai_file" => {
            let params: ::task_core::GetInstructionsForMainAiFileParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_instructions_for_main_ai_file(params).await {
                Ok(instructions) => match serde_json::to_value(instructions) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "create_main_ai_file" => {
            let params: ::task_core::CreateMainAiFileParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.create_main_ai_file(params).await {
                Ok(file_data) => match serde_json::to_value(file_data) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_workspace_manifest" => {
            let params: ::task_core::GetWorkspaceManifestParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return McpError::from(e).to_json_rpc_error(id),
            };
            match handler.get_workspace_manifest(params).await {
                Ok(manifest) => match serde_json::to_value(manifest) {
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
async fn sse_handler<R: TaskRepository + Send + Sync + 'static, M: TaskMessageRepository + Send + Sync + 'static>(
    State(_state): State<Arc<McpServerState<R, M>>>,
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
                    "assign_task", "archive_task", "health_check",
                    "discover_work", "claim_task", "release_task", 
                    "start_work_session", "end_work_session",
                    "create_task_message", "get_task_messages",
                    "get_setup_instructions", "get_agentic_workflow_description",
                    "register_agent", "get_instructions_for_main_ai_file",
                    "create_main_ai_file", "get_workspace_manifest"
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
async fn rpc_handler<R: TaskRepository + Send + Sync + 'static, M: TaskMessageRepository + Send + Sync + 'static>(
    State(state): State<Arc<McpServerState<R, M>>>,
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
    use ::task_core::{Task, NewTask, UpdateTask, TaskFilter, TaskState, RepositoryStats, TaskMessage};
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
            async fn discover_work(&self, agent_name: &str, capabilities: &[String], max_tasks: u32) -> Result<Vec<Task>>;
            async fn claim_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;
            async fn release_task(&self, task_id: i32, agent_name: &str) -> Result<Task>;
            async fn start_work_session(&self, task_id: i32, agent_name: &str) -> Result<i32>;
            async fn end_work_session(&self, session_id: i32, notes: Option<String>, productivity_score: Option<f64>) -> Result<()>;
        }
    }
    
    // Simple mock for testing server creation
    struct SimpleTestMessageRepository;
    
    #[async_trait]
    impl TaskMessageRepository for SimpleTestMessageRepository {
        async fn create_message(
            &self,
            task_code: &str,
            author_agent_name: &str,
            _target_agent_name: Option<&str>,
            message_type: &str,
            content: &str,
            reply_to_message_id: Option<i32>,
        ) -> Result<TaskMessage> {
            Ok(TaskMessage {
                id: 1,
                task_code: task_code.to_string(),
                author_agent_name: author_agent_name.to_string(),
                target_agent_name: None,
                message_type: message_type.to_string(),
                created_at: chrono::Utc::now(),
                content: content.to_string(),
                reply_to_message_id,
            })
        }
        
        async fn get_messages(
            &self,
            _task_code: &str,
            _author_agent_name: Option<&str>,
            _target_agent_name: Option<&str>,
            _message_type: Option<&str>,
            _reply_to_message_id: Option<i32>,
            _limit: Option<u32>,
        ) -> Result<Vec<TaskMessage>> {
            Ok(vec![])
        }
        
        async fn get_message_by_id(&self, _message_id: i32) -> Result<Option<TaskMessage>> {
            Ok(None)
        }
    }
    
    #[test]
    fn test_server_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let mock_message_repo = Arc::new(SimpleTestMessageRepository);
        let _server = McpServer::new(mock_repo, mock_message_repo);
        // Basic test that server can be created
        assert!(true);
    }
}