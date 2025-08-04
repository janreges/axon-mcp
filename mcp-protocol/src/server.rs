//! MCP v2 Server with Multiple Transport Support
//!
//! Implements the MCP server supporting both modern Streamable HTTP transport (2025-06-18)
//! and legacy Server-Sent Events for backward compatibility.

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    middleware,
    response::Sse,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::info;

use crate::{error::McpError, handler::McpTaskHandler, serialization::*};
use ::task_core::{
    ClaimTaskParams, CreateTaskMessageParams, DiscoverWorkParams, EndWorkSessionParams,
    GetTaskMessagesParams, ProtocolHandler, ReleaseTaskParams, StartWorkSessionParams,
    TaskMessageRepository, TaskRepository, WorkspaceContextRepository,
};

/// MCP Protocol Version as required by 2025-06-18 specification
const MCP_PROTOCOL_VERSION: &str = "2025-06-18";

/// Shared server state for handlers
#[derive(Clone)]
pub struct McpServerState<R, M, W> {
    pub handler: McpTaskHandler<R, M, W>,
}

/// MCP Server with multiple transport support
pub struct McpServer<R, M, W> {
    handler: McpTaskHandler<R, M, W>,
}

impl<
        R: TaskRepository + Send + Sync + 'static,
        M: TaskMessageRepository + Send + Sync + 'static,
        W: WorkspaceContextRepository + Send + Sync + 'static,
    > McpServer<R, M, W>
{
    /// Create new MCP server for local usage (no authentication)
    pub fn new(
        repository: Arc<R>,
        message_repository: Arc<M>,
        workspace_context_repository: Arc<W>,
        project_root: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            handler: McpTaskHandler::new(
                repository,
                message_repository,
                workspace_context_repository,
                project_root,
            ),
        }
    }

    /// Start the MCP server for local PC usage
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();

        let socket_addr: SocketAddr = addr
            .parse()
            .map_err(|e| format!("Invalid address '{addr}': {e}"))?;

        info!("Starting MCP server on {}", socket_addr);

        let listener = tokio::net::TcpListener::bind(socket_addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the router with all endpoints
    fn create_router(self) -> Router {
        let state = Arc::new(McpServerState {
            handler: self.handler,
        });

        Router::new()
            .route("/mcp", post(rpc_handler)) // MCP 2025-06-18 Streamable HTTP transport
            .route("/mcp/v1", get(sse_handler)) // Legacy SSE support (deprecated)
            .route("/mcp/v1/rpc", post(rpc_handler)) // Legacy RPC support (deprecated)
            .route("/health", get(health_handler))
            .layer(middleware::from_fn(crate::request_logger::mcp_request_logging_middleware))
            .with_state(state)
    }
}

/// Execute MCP method - shared logic for both server instances and handlers
async fn execute_mcp_method<
    R: TaskRepository + Send + Sync,
    M: TaskMessageRepository + Send + Sync,
    W: WorkspaceContextRepository + Send + Sync,
>(
    handler: &McpTaskHandler<R, M, W>,
    method: &str,
    params: Value,
    id: Option<Value>,
) -> Value {
    match method {
        "create_task" => {
            let params: CreateTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.create_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "update_task" => {
            let params: UpdateTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.update_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "set_task_state" => {
            let params: SetStateParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.set_task_state(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_task_by_id" => {
            let params: GetTaskByIdParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.get_task_by_id(params).await {
                Ok(Some(task)) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Ok(None) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "get_task_by_code" => {
            let params: GetTaskByCodeParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.get_task_by_code(params).await {
                Ok(Some(task)) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Ok(None) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "list_tasks" => {
            let params: ListTasksParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.list_tasks(params).await {
                Ok(tasks) => {
                    let task_values: Result<Vec<_>, _> =
                        tasks.iter().map(serialize_task_for_mcp).collect();
                    match task_values {
                        Ok(values) => create_success_response(id, Value::Array(values)),
                        Err(e) => e.to_json_rpc_error(id),
                    }
                }
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "assign_task" => {
            let params: AssignTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.assign_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "archive_task" => {
            let params: ArchiveTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.archive_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "health_check" => match handler.health_check().await {
            Ok(health) => match serde_json::to_value(health) {
                Ok(value) => create_success_response(id, value),
                Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
            },
            Err(e) => McpError::from(e).to_json_rpc_error(id),
        },
        // MCP v2 Advanced Multi-Agent Functions
        "discover_work" => {
            let params: DiscoverWorkParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.discover_work(params).await {
                Ok(tasks) => {
                    let task_values: Result<Vec<_>, _> =
                        tasks.iter().map(serialize_task_for_mcp).collect();
                    match task_values {
                        Ok(values) => create_success_response(id, Value::Array(values)),
                        Err(e) => e.to_json_rpc_error(id),
                    }
                }
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "claim_task" => {
            let params: ClaimTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.claim_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "release_task" => {
            let params: ReleaseTaskParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.release_task(params).await {
                Ok(task) => match serialize_task_for_mcp(&task) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => e.to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "start_work_session" => {
            let params: StartWorkSessionParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.start_work_session(params).await {
                Ok(session_info) => match serde_json::to_value(session_info) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "end_work_session" => {
            let params: EndWorkSessionParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.end_work_session(params).await {
                Ok(()) => create_success_response(id, Value::Null),
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "cleanup_timed_out_tasks" => {
            let params: ::task_core::CleanupTimedOutTasksParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.cleanup_timed_out_tasks(params).await {
                Ok(tasks) => {
                    let task_values: Result<Vec<_>, _> =
                        tasks.iter().map(serialize_task_for_mcp).collect();
                    match task_values {
                        Ok(values) => create_success_response(id, Value::Array(values)),
                        Err(e) => e.to_json_rpc_error(id),
                    }
                }
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        // Task Messaging Functions
        "create_task_message" => {
            let params: CreateTaskMessageParams = match deserialize_mcp_params(params) {
                Ok(p) => p,
                Err(e) => return e.to_json_rpc_error(id),
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
                Err(e) => return e.to_json_rpc_error(id),
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
            let params: ::task_core::GetSetupInstructionsParams =
                match deserialize_mcp_params(params) {
                    Ok(p) => p,
                    Err(e) => return e.to_json_rpc_error(id),
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
            let params: ::task_core::GetAgenticWorkflowDescriptionParams =
                match deserialize_mcp_params(params) {
                    Ok(p) => p,
                    Err(e) => return e.to_json_rpc_error(id),
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
                Err(e) => return e.to_json_rpc_error(id),
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
            let params: ::task_core::GetInstructionsForMainAiFileParams =
                match deserialize_mcp_params(params) {
                    Ok(p) => p,
                    Err(e) => return e.to_json_rpc_error(id),
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
                Err(e) => return e.to_json_rpc_error(id),
            };
            match handler.create_main_ai_file(params).await {
                Ok(file_data) => match serde_json::to_value(file_data) {
                    Ok(value) => create_success_response(id, value),
                    Err(e) => McpError::Serialization(e.to_string()).to_json_rpc_error(id),
                },
                Err(e) => McpError::from(e).to_json_rpc_error(id),
            }
        }
        "tools/list" => {
            // Return list of all available tools per MCP specification
            let tools_list = json!({
                "tools": [
                    {
                        "name": "create_task",
                        "description": "Create a new task",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {"type": "string"},
                                "name": {"type": "string"},
                                "description": {"type": "string"},
                                "owner_agent_name": {"type": "string"}
                            },
                            "required": ["code", "name", "description", "owner_agent_name"]
                        }
                    },
                    {
                        "name": "update_task",
                        "description": "Update an existing task",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "integer"},
                                "name": {"type": "string"},
                                "description": {"type": "string"}
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "set_task_state",
                        "description": "Set task state",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "integer"},
                                "state": {"type": "string", "enum": ["Created", "InProgress", "Blocked", "Review", "Done", "Archived"]}
                            },
                            "required": ["id", "state"]
                        }
                    },
                    {
                        "name": "get_task_by_id",
                        "description": "Get task by ID",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "integer"}
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "get_task_by_code",
                        "description": "Get task by code",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "code": {"type": "string"}
                            },
                            "required": ["code"]
                        }
                    },
                    {
                        "name": "list_tasks",
                        "description": "List tasks with optional filtering",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "owner_agent_name": {"type": "string"},
                                "state": {"type": "string"},
                                "limit": {"type": "integer"},
                                "offset": {"type": "integer"}
                            }
                        }
                    },
                    {
                        "name": "assign_task",
                        "description": "Assign task to a different agent",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "integer"},
                                "new_owner_agent_name": {"type": "string"}
                            },
                            "required": ["id", "new_owner_agent_name"]
                        }
                    },
                    {
                        "name": "archive_task",
                        "description": "Archive a completed task",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "integer"}
                            },
                            "required": ["id"]
                        }
                    },
                    {
                        "name": "health_check",
                        "description": "Check server health",
                        "inputSchema": {
                            "type": "object"
                        }
                    },
                    {
                        "name": "discover_work",
                        "description": "Discover available work based on agent capabilities",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_name": {"type": "string"},
                                "capabilities": {"type": "array", "items": {"type": "string"}},
                                "max_tasks": {"type": "integer"}
                            },
                            "required": ["agent_name", "capabilities"]
                        }
                    },
                    {
                        "name": "claim_task",
                        "description": "Atomically claim a task for execution",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task_id": {"type": "integer"},
                                "agent_name": {"type": "string"}
                            },
                            "required": ["task_id", "agent_name"]
                        }
                    },
                    {
                        "name": "release_task",
                        "description": "Release a claimed task back to the pool",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task_id": {"type": "integer"},
                                "agent_name": {"type": "string"}
                            },
                            "required": ["task_id", "agent_name"]
                        }
                    },
                    {
                        "name": "start_work_session",
                        "description": "Start a work session for task tracking",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task_id": {"type": "integer"},
                                "agent_name": {"type": "string"}
                            },
                            "required": ["task_id", "agent_name"]
                        }
                    },
                    {
                        "name": "end_work_session",
                        "description": "End a work session with productivity metrics",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "session_id": {"type": "integer"},
                                "productivity_score": {"type": "number"}
                            },
                            "required": ["session_id"]
                        }
                    },
                    {
                        "name": "create_task_message",
                        "description": "Create a message within a task context",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task_code": {"type": "string"},
                                "author_agent_name": {"type": "string"},
                                "target_agent_name": {"type": "string"},
                                "message_type": {"type": "string"},
                                "content": {"type": "string"},
                                "reply_to_message_id": {"type": "integer"}
                            },
                            "required": ["task_code", "author_agent_name", "message_type", "content"]
                        }
                    },
                    {
                        "name": "get_task_messages",
                        "description": "Get messages from a task with filtering",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "task_code": {"type": "string"},
                                "author_agent_name": {"type": "string"},
                                "target_agent_name": {"type": "string"},
                                "message_type": {"type": "string"},
                                "reply_to_message_id": {"type": "integer"},
                                "limit": {"type": "integer"}
                            },
                            "required": ["task_code"]
                        }
                    },
                    {
                        "name": "get_setup_instructions",
                        "description": "Generate AI workspace setup instructions",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "ai_tool_type": {"type": "string"}
                            },
                            "required": ["ai_tool_type"]
                        }
                    },
                    {
                        "name": "get_agentic_workflow_description",
                        "description": "Get recommended agent workflow for workspace",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "requested_agent_count": {"type": "integer"}
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "register_agent",
                        "description": "Register an AI agent in the workspace",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "agent_name": {"type": "string"},
                                "agent_type": {"type": "string"},
                                "capabilities": {"type": "array", "items": {"type": "string"}},
                                "description": {"type": "string"}
                            },
                            "required": ["agent_name", "agent_type", "capabilities"]
                        }
                    },
                    {
                        "name": "get_instructions_for_main_ai_file",
                        "description": "Get instructions for creating main AI coordination file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_type": {"type": "string"}
                            },
                            "required": []
                        }
                    },
                    {
                        "name": "create_main_ai_file",
                        "description": "Create the main AI coordination file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "content": {"type": "string"}
                            },
                            "required": ["content"]
                        }
                    }
                ]
            });
            create_success_response(id, tools_list)
        }
        _ => McpError::Protocol(format!("Unknown method: {method}")).to_json_rpc_error(id),
    }
}

/// SSE endpoint for MCP communication
async fn sse_handler<
    R: TaskRepository + Send + Sync + 'static,
    M: TaskMessageRepository + Send + Sync + 'static,
    W: WorkspaceContextRepository + Send + Sync + 'static,
>(
    State(_state): State<Arc<McpServerState<R, M, W>>>,
) -> Result<Sse<UnboundedReceiverStream<Result<axum::response::sse::Event, axum::Error>>>, StatusCode>
{
    let (tx, rx) = mpsc::unbounded_channel();

    // Send initial connection event (this is legacy SSE, not proper MCP)
    let welcome_event = axum::response::sse::Event::default().data(
        json!({
            "jsonrpc": "2.0",
            "method": "connection_established",
            "params": {
                "server": "mcp-task-server",
                "version": env!("CARGO_PKG_VERSION"),
                "protocol_version": MCP_PROTOCOL_VERSION,
                "note": "This is legacy SSE transport. For proper MCP, use HTTP POST to /mcp endpoint with initialize request."
            }
        })
        .to_string(),
    );

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
async fn rpc_handler<
    R: TaskRepository + Send + Sync + 'static,
    M: TaskMessageRepository + Send + Sync + 'static,
    W: WorkspaceContextRepository + Send + Sync + 'static,
>(
    State(state): State<Arc<McpServerState<R, M, W>>>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<(HeaderMap, Json<Value>), StatusCode> {
    info!("Received RPC request: {}", request);

    // Extract ID first for error responses
    let id = request.get("id").cloned();

    // Validate MCP-Protocol-Version header (required by 2025-06-18 spec)
    let protocol_version = headers
        .get("MCP-Protocol-Version")
        .or_else(|| headers.get("mcp-protocol-version")) // Try lowercase variant
        .and_then(|v| v.to_str().ok());

    // Set response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::HeaderName::from_static("mcp-protocol-version"),
        MCP_PROTOCOL_VERSION.parse().unwrap(),
    );
    response_headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

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
            let error = McpError::Protocol(format!("Unsupported MCP-Protocol-Version: {version}. Supported versions: {MCP_PROTOCOL_VERSION}, 2025-03-26"));
            return Ok((response_headers, Json(error.to_json_rpc_error(id))));
        }
        None => {
            // Missing header - default to backward compatibility
            info!(
                "Missing MCP-Protocol-Version header, defaulting to 2025-03-26 compatibility mode"
            );
        }
    }

    // Validate that request is not a JSON-RPC batch (forbidden in 2025-06-18)
    if request.is_array() {
        let error = McpError::Protocol(
            "JSON-RPC batching is not supported in MCP 2025-06-18 specification".to_string(),
        );
        return Ok((response_headers, Json(error.to_json_rpc_error(id))));
    }

    // Parse JSON-RPC request - return JSON-RPC errors instead of HTTP errors
    let method = match request.get("method").and_then(|v| v.as_str()) {
        Some(method) => method,
        None => {
            let error = McpError::Protocol(
                "Missing or invalid 'method' field in JSON-RPC request".to_string(),
            );
            return Ok((response_headers, Json(error.to_json_rpc_error(id))));
        }
    };

    let params = request.get("params").unwrap_or(&Value::Null).clone();

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
    use ::task_core::error::Result;
    use ::task_core::workspace_setup::WorkspaceContext;
    use ::task_core::{
        NewTask, RepositoryStats, Task, TaskFilter, TaskMessage, TaskState, UpdateTask,
        WorkspaceContextRepository,
    };
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;

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

    // Simple mock workspace context repository for testing
    struct SimpleTestWorkspaceContextRepository;

    #[async_trait]
    impl WorkspaceContextRepository for SimpleTestWorkspaceContextRepository {
        async fn create(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
            Ok(context)
        }

        async fn get_by_id(&self, _workspace_id: &str) -> Result<Option<WorkspaceContext>> {
            Ok(None)
        }

        async fn update(&self, context: WorkspaceContext) -> Result<WorkspaceContext> {
            Ok(context)
        }

        async fn delete(&self, _workspace_id: &str) -> Result<()> {
            Ok(())
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_server_creation() {
        let mock_repo = Arc::new(MockTestRepository::new());
        let mock_message_repo = Arc::new(SimpleTestMessageRepository);
        let mock_workspace_repo = Arc::new(SimpleTestWorkspaceContextRepository);
        let _server = McpServer::new(mock_repo, mock_message_repo, mock_workspace_repo, None);
        // Basic test that server can be created
        // Test passes if server creation doesn't panic
    }
}
