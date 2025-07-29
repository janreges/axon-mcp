//! STDIO Transport for MCP Server
//! 
//! Implements MCP communication over stdin/stdout following the MCP specification.
//! Uses line-based JSON-RPC 2.0 protocol with proper initialize/initialized handshake.

use std::sync::Arc;
use anyhow::{Context, Result};
use mcp_protocol::{McpTaskHandler, McpError};
use serde_json::{json, Value};
use task_core::{TaskRepository, ProtocolHandler};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, error, debug, warn};

/// MCP protocol state tracking
#[derive(Debug, PartialEq)]
enum McpState {
    /// Waiting for initialize request from client
    WaitingForInitialize,
    /// Initialize request received, sent response, waiting for initialized notification
    WaitingForInitialized,
    /// Fully initialized and ready to process requests
    Ready,
}

/// STDIO MCP Server with proper protocol state management
pub struct StdioMcpServer<R> {
    handler: McpTaskHandler<R>,
    state: McpState,
}

impl<R: TaskRepository + Send + Sync + 'static> StdioMcpServer<R> {
    /// Create new STDIO MCP server
    pub fn new(repository: Arc<R>) -> Self {
        Self {
            handler: McpTaskHandler::new(repository),
            state: McpState::WaitingForInitialize,
        }
    }
    
    /// Start the STDIO MCP server
    pub async fn serve(mut self) -> Result<()> {
        info!("Starting MCP server in STDIO mode - waiting for initialize request");
        
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        
        loop {
            line.clear();
            
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF reached
                    info!("STDIN closed, shutting down MCP server");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    
                    debug!("Received line: {}", trimmed);
                    
                    // Process the JSON-RPC request/notification
                    match self.process_message(trimmed).await {
                        Ok(Some(response)) => {
                            // Send response to stdout
                            let response_json = serde_json::to_string(&response)
                                .context("Failed to serialize JSON-RPC response")?;
                            
                            stdout.write_all(response_json.as_bytes()).await
                                .context("Failed to write response to stdout")?;
                            stdout.write_all(b"\n").await
                                .context("Failed to write newline to stdout")?;
                            stdout.flush().await
                                .context("Failed to flush stdout")?;
                            
                            debug!("Sent JSON-RPC response: {}", response_json);
                        }
                        Ok(None) => {
                            // Notification processed, no response needed
                            debug!("Processed notification successfully");
                        }
                        Err(e) => {
                            error!("Error processing message: {}", e);
                            
                            // Try to parse the line to get the ID for error response
                            let id = self.extract_id_from_line(trimmed);
                            
                            // Create proper JSON-RPC error response
                            let error_response = self.create_error_response(e, id);
                            
                            let error_json = serde_json::to_string(&error_response)
                                .unwrap_or_else(|_| r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"},"id":null}"#.to_string());
                            
                            let _ = stdout.write_all(error_json.as_bytes()).await;
                            let _ = stdout.write_all(b"\n").await;
                            let _ = stdout.flush().await;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
        
        info!("STDIO MCP server shutdown complete");
        Ok(())
    }
    
    /// Process a message - could be request or notification
    async fn process_message(&mut self, line: &str) -> Result<Option<Value>> {
        // Parse JSON-RPC message
        let message: Value = serde_json::from_str(line)
            .context("Failed to parse JSON-RPC message")?;
        
        // Validate JSON-RPC 2.0 format
        if message.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
            return Err(anyhow::anyhow!("Invalid JSON-RPC version"));
        }
        
        let method = message.get("method")
            .and_then(|v| v.as_str())
            .context("Missing or invalid 'method' field")?;
        
        let id = message.get("id").cloned();
        let params = message.get("params").unwrap_or(&Value::Null).clone();
        
        // Check if this is a notification (no ID) or a request (has ID)
        let is_notification = id.is_none();
        
        // Handle based on current protocol state
        match (&self.state, method) {
            (McpState::WaitingForInitialize, "initialize") => {
                if is_notification {
                    return Err(anyhow::anyhow!("Initialize must be a request, not a notification"));
                }
                
                info!("Received initialize request");
                self.state = McpState::WaitingForInitialized;
                
                // Return initialize response
                Ok(Some(json!({
                    "jsonrpc": "2.0",
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {
                                "create_task": {
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
                                "update_task": {
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
                                "set_task_state": {
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
                                "get_task_by_id": {
                                    "description": "Get task by ID",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"}
                                        },
                                        "required": ["id"]
                                    }
                                },
                                "get_task_by_code": {
                                    "description": "Get task by code",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "code": {"type": "string"}
                                        },
                                        "required": ["code"]
                                    }
                                },
                                "list_tasks": {
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
                                "assign_task": {
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
                                "archive_task": {
                                    "description": "Archive a completed task",
                                    "inputSchema": {
                                        "type": "object",
                                        "properties": {
                                            "id": {"type": "integer"}
                                        },
                                        "required": ["id"]
                                    }
                                },
                                "health_check": {
                                    "description": "Check server health",
                                    "inputSchema": {
                                        "type": "object"
                                    }
                                }
                            }
                        },
                        "serverInfo": {
                            "name": "mcp-task-server",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    },
                    "id": id
                })))
            }
            
            (McpState::WaitingForInitialized, "notifications/initialized") => {
                if !is_notification {
                    return Err(anyhow::anyhow!("Initialized must be a notification, not a request"));
                }
                
                info!("Received initialized notification - server is ready");
                self.state = McpState::Ready;
                
                // No response for notifications
                Ok(None)
            }
            
            (McpState::Ready, _) => {
                // Server is ready, process normal requests
                if is_notification {
                    // Handle notifications (no response needed)
                    match method {
                        "notifications/cancelled" => {
                            debug!("Received cancelled notification");
                            Ok(None)
                        }
                        _ => {
                            warn!("Unknown notification method: {}", method);
                            Ok(None)
                        }
                    }
                } else {
                    // Handle requests (response needed)
                    match self.execute_tool_call(method, params).await {
                        Ok(result) => {
                            Ok(Some(json!({
                                "jsonrpc": "2.0",
                                "result": result,
                                "id": id
                            })))
                        }
                        Err(e) => {
                            let mcp_error = McpError::from(e);
                            Ok(Some(mcp_error.to_json_rpc_error(id)))
                        }
                    }
                }
            }
            
            _ => {
                // Invalid state transition
                Err(anyhow::anyhow!(
                    "Invalid method '{}' for current state {:?}", 
                    method, 
                    self.state
                ))
            }
        }
    }
    
    /// Execute a tool call (task management operation)
    async fn execute_tool_call(&self, method: &str, params: Value) -> Result<Value, anyhow::Error> {
        
        match method {
            "tools/call" => {
                // Extract tool name and arguments from MCP tools/call format
                let tool_name = params.get("name")
                    .and_then(|v| v.as_str())
                    .context("Missing tool name in tools/call")?;
                    
                let arguments = params.get("arguments")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                
                self.execute_task_operation(tool_name, arguments).await
            }
            _ => {
                // Direct method calls for compatibility
                self.execute_task_operation(method, params).await
            }
        }
    }
    
    /// Execute task management operations
    async fn execute_task_operation(&self, operation: &str, params: Value) -> Result<Value, anyhow::Error> {
        use mcp_protocol::{serialize_task_for_mcp, CreateTaskParams, UpdateTaskParams, SetStateParams, 
                           GetTaskByIdParams, GetTaskByCodeParams, ListTasksParams, AssignTaskParams, ArchiveTaskParams};
        
        match operation {
            "create_task" => {
                let params: CreateTaskParams = serde_json::from_value(params)
                    .context("Invalid create_task parameters")?;
                let task = self.handler.create_task(params).await?;
                serialize_task_for_mcp(&task).map_err(anyhow::Error::from)
            }
            "update_task" => {
                let params: UpdateTaskParams = serde_json::from_value(params)
                    .context("Invalid update_task parameters")?;
                let task = self.handler.update_task(params).await?;
                serialize_task_for_mcp(&task).map_err(anyhow::Error::from)
            }
            "set_task_state" => {
                let params: SetStateParams = serde_json::from_value(params)
                    .context("Invalid set_task_state parameters")?;
                let task = self.handler.set_task_state(params).await?;
                serialize_task_for_mcp(&task).map_err(anyhow::Error::from)
            }
            "get_task_by_id" => {
                let params: GetTaskByIdParams = serde_json::from_value(params)
                    .context("Invalid get_task_by_id parameters")?;
                match self.handler.get_task_by_id(params).await? {
                    Some(task) => serialize_task_for_mcp(&task).map_err(anyhow::Error::from),
                    None => Ok(Value::Null),
                }
            }
            "get_task_by_code" => {
                let params: GetTaskByCodeParams = serde_json::from_value(params)
                    .context("Invalid get_task_by_code parameters")?;
                match self.handler.get_task_by_code(params).await? {
                    Some(task) => serialize_task_for_mcp(&task).map_err(anyhow::Error::from),
                    None => Ok(Value::Null),
                }
            }
            "list_tasks" => {
                let params: ListTasksParams = serde_json::from_value(params)
                    .context("Invalid list_tasks parameters")?;
                let tasks = self.handler.list_tasks(params).await?;
                let task_values: Result<Vec<_>, McpError> = tasks.iter()
                    .map(serialize_task_for_mcp)
                    .collect();
                let task_values = task_values.map_err(anyhow::Error::from)?;
                Ok(Value::Array(task_values))
            }
            "assign_task" => {
                let params: AssignTaskParams = serde_json::from_value(params)
                    .context("Invalid assign_task parameters")?;
                let task = self.handler.assign_task(params).await?;
                serialize_task_for_mcp(&task).map_err(anyhow::Error::from)
            }
            "archive_task" => {
                let params: ArchiveTaskParams = serde_json::from_value(params)
                    .context("Invalid archive_task parameters")?;
                let task = self.handler.archive_task(params).await?;
                serialize_task_for_mcp(&task).map_err(anyhow::Error::from)
            }
            "health_check" => {
                let health = self.handler.health_check().await?;
                Ok(serde_json::to_value(health)?)
            }
            _ => {
                Err(anyhow::anyhow!("Unknown operation: {}", operation))
            }
        }
    }
    
    /// Extract ID from a malformed JSON line for error responses
    fn extract_id_from_line(&self, line: &str) -> Option<Value> {
        // Try to parse as JSON and extract ID
        if let Ok(json) = serde_json::from_str::<Value>(line) {
            json.get("id").cloned()
        } else {
            None
        }
    }
    
    /// Create a proper JSON-RPC error response
    fn create_error_response(&self, error: anyhow::Error, id: Option<Value>) -> Value {
        let mcp_error = McpError::from(error);
        mcp_error.to_json_rpc_error(id)
    }
}