# PROTOCOL01: Implement Core MCP Protocol Handlers

## Objective
Implement the core MCP protocol handlers in the mcp-protocol crate, mapping JSON-RPC requests to repository methods and ensuring full MCP v1 compatibility.

## Implementation Details

### 1. Create Protocol Handler Implementation
In `mcp-protocol/src/handler.rs`:

```rust
use async_trait::async_trait;
use core::{
    error::Result,
    models::*,
    protocol::*,
    repository::TaskRepository,
};
use serde_json::Value;
use std::sync::Arc;

pub struct McpProtocolHandler<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> McpProtocolHandler<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl<R: TaskRepository> ProtocolHandler for McpProtocolHandler<R> {
    // ===== Original MCP v1 Methods =====
    
    async fn create_task(&self, params: CreateTaskParams) -> Result<Task> {
        let new_task = NewTask {
            code: params.code,
            name: params.name,
            description: params.description,
            owner_agent_name: params.owner_agent_name,
        };
        
        self.repository.create_task(new_task).await
    }
    
    async fn update_task(&self, params: UpdateTaskParams) -> Result<Task> {
        // First get the task
        let mut task = self.repository
            .get_task_by_code(&params.code)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", params.code)))?;
        
        // Update fields if provided
        if let Some(name) = params.name {
            task.name = name;
        }
        if let Some(description) = params.description {
            task.description = description;
        }
        if let Some(owner) = params.owner_agent_name {
            task.owner_agent_name = owner;
        }
        
        self.repository.update_task(task).await
    }
    
    async fn get_task(&self, params: GetTaskParams) -> Result<Option<Task>> {
        self.repository.get_task_by_code(&params.code).await
    }
    
    async fn list_tasks(&self, params: ListTasksParams) -> Result<Vec<Task>> {
        let filter = TaskFilter {
            owner_agent_name: params.owner_agent_name,
            state: params.state.and_then(|s| TaskState::try_from(s.as_str()).ok()),
            priority_min: params.priority_min,
            limit: params.limit,
            offset: params.offset,
        };
        
        self.repository.list_tasks(filter).await
    }
    
    async fn delete_task(&self, params: DeleteTaskParams) -> Result<()> {
        self.repository.delete_task(&params.code).await
    }
    
    async fn set_task_state(&self, params: SetTaskStateParams) -> Result<Task> {
        let state = TaskState::try_from(params.state.as_str())?;
        self.repository.set_task_state(&params.code, state).await
    }
    
    async fn search_tasks(&self, params: SearchTasksParams) -> Result<Vec<Task>> {
        self.repository.search_tasks(&params.query).await
    }
    
    // ===== New MCP v2 Methods =====
    
    async fn discover_work(&self, params: DiscoverWorkParams) -> Result<Vec<Task>> {
        // Get agent to determine capabilities
        let agent = self.repository
            .get_agent(&params.agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.agent_name)))?;
        
        let discovery_params = WorkDiscoveryParams {
            agent_name: params.agent_name,
            capabilities: agent.capabilities,
            max_tasks: params.max_tasks.unwrap_or(10),
            include_types: vec![
                TaskState::Created,
                TaskState::InProgress,
                TaskState::Review,
                TaskState::PendingHandoff,
            ],
            exclude_codes: vec![],
            min_priority: None,
        };
        
        self.repository.discover_work(discovery_params).await
    }
    
    async fn assign_task(&self, params: AssignTaskParams) -> Result<Task> {
        // Validate agent exists
        let agent = self.repository
            .get_agent(&params.agent_name)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Agent {} not found", params.agent_name)))?;
        
        // Get task
        let mut task = self.repository
            .get_task_by_code(&params.task_code)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Task {} not found", params.task_code)))?;
        
        // Update owner
        task.owner_agent_name = params.agent_name;
        self.repository.update_task(task).await
    }
    
    async fn report_progress(&self, params: ReportProgressParams) -> Result<()> {
        // This could update a progress field or log an event
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: "task_progress".to_string(),
            actor_type: ActorType::Agent,
            actor_id: params.agent_name,
            task_code: Some(params.task_code.clone()),
            payload: serde_json::json!({
                "progress_percentage": params.progress_percentage,
                "status_message": params.status_message,
            }),
            correlation_id: None,
        };
        
        self.repository.log_event(event).await
    }
    
    async fn add_task_message(&self, params: AddTaskMessageParams) -> Result<TaskMessage> {
        let message = NewTaskMessage {
            task_code: params.task_code,
            author_agent_name: params.author_agent_name,
            message_type: params.message_type,
            content: params.content,
            reply_to_message_id: params.reply_to_message_id,
        };
        
        self.repository.add_task_message(message).await
    }
    
    async fn get_task_messages(&self, params: GetTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        let filter = MessageFilter {
            task_code: Some(params.task_code),
            message_types: params.message_types,
            since: params.since.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            author_agent_name: None,
            limit: params.limit,
            offset: None,
        };
        
        self.repository.get_task_messages(filter).await
    }
    
    async fn search_task_messages(&self, params: SearchTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        let query = MessageSearchQuery {
            query: params.query,
            task_codes: params.task_codes,
            message_types: params.message_types,
            limit: params.limit,
        };
        
        self.repository.search_task_messages(query).await
    }
}
```

### 2. Create JSON-RPC Request Router
In `mcp-protocol/src/router.rs`:

```rust
use crate::handler::McpProtocolHandler;
use core::{error::*, protocol::*, repository::TaskRepository};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn invalid_request(message: &str) -> Self {
        Self {
            code: -32600,
            message: message.to_string(),
            data: None,
        }
    }
    
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }
    
    pub fn invalid_params(message: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", message),
            data: None,
        }
    }
    
    pub fn internal_error(error: &TaskError) -> Self {
        Self {
            code: match error {
                TaskError::NotFound(_) => -32001,
                TaskError::AlreadyExists(_) => -32002,
                TaskError::Validation(_) => -32003,
                _ => -32603,
            },
            message: error.to_string(),
            data: None,
        }
    }
}

pub struct JsonRpcRouter<R: TaskRepository> {
    handler: McpProtocolHandler<R>,
}

impl<R: TaskRepository> JsonRpcRouter<R> {
    pub fn new(handler: McpProtocolHandler<R>) -> Self {
        Self { handler }
    }
    
    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError::invalid_request("Invalid JSON-RPC version")),
                id: request.id,
            };
        }
        
        // Route to appropriate handler
        let result = match request.method.as_str() {
            // Original MCP v1 methods
            "task/create" => self.handle_create_task(request.params).await,
            "task/update" => self.handle_update_task(request.params).await,
            "task/get" => self.handle_get_task(request.params).await,
            "task/list" => self.handle_list_tasks(request.params).await,
            "task/delete" => self.handle_delete_task(request.params).await,
            "task/setState" => self.handle_set_task_state(request.params).await,
            "task/search" => self.handle_search_tasks(request.params).await,
            
            // New MCP v2 methods
            "work/discover" => self.handle_discover_work(request.params).await,
            "task/assign" => self.handle_assign_task(request.params).await,
            "task/reportProgress" => self.handle_report_progress(request.params).await,
            "message/add" => self.handle_add_message(request.params).await,
            "message/list" => self.handle_get_messages(request.params).await,
            "message/search" => self.handle_search_messages(request.params).await,
            "knowledge/create" => self.handle_create_knowledge(request.params).await,
            "knowledge/get" => self.handle_get_knowledge(request.params).await,
            "knowledge/search" => self.handle_search_knowledge(request.params).await,
            "agent/register" => self.handle_register_agent(request.params).await,
            "agent/get" => self.handle_get_agent(request.params).await,
            "agent/list" => self.handle_list_agents(request.params).await,
            "agent/heartbeat" => self.handle_heartbeat(request.params).await,
            "help/create" => self.handle_create_help_request(request.params).await,
            "help/list" => self.handle_list_help_requests(request.params).await,
            "help/claim" => self.handle_claim_help_request(request.params).await,
            "help/resolve" => self.handle_resolve_help_request(request.params).await,
            "workflow/create" => self.handle_create_workflow(request.params).await,
            "workflow/assign" => self.handle_assign_workflow(request.params).await,
            "workflow/advance" => self.handle_advance_workflow(request.params).await,
            "handoff/create" => self.handle_create_handoff(request.params).await,
            "handoff/accept" => self.handle_accept_handoff(request.params).await,
            "task/decompose" => self.handle_decompose_task(request.params).await,
            "metrics/tasks" => self.handle_get_task_metrics(request.params).await,
            "metrics/agent" => self.handle_get_agent_metrics(request.params).await,
            "metrics/system" => self.handle_get_system_metrics(request.params).await,
            
            _ => Err(JsonRpcError::method_not_found(&request.method)),
        };
        
        match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(value),
                error: None,
                id: request.id,
            },
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(error),
                id: request.id,
            },
        }
    }
    
    // Handler implementations
    async fn handle_create_task(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let create_params: CreateTaskParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.create_task(create_params).await {
            Ok(task) => Ok(serde_json::to_value(task).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_update_task(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let update_params: UpdateTaskParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.update_task(update_params).await {
            Ok(task) => Ok(serde_json::to_value(task).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_get_task(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let get_params: GetTaskParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.get_task(get_params).await {
            Ok(task) => Ok(serde_json::to_value(task).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    // ... implement remaining handlers following the same pattern ...
}
```

### 3. Create SSE Transport Implementation
In `mcp-protocol/src/transport/sse.rs`:

```rust
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::Stream;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::router::{JsonRpcRequest, JsonRpcResponse, JsonRpcRouter};
use core::repository::TaskRepository;

pub struct SseTransport<R: TaskRepository> {
    router: Arc<JsonRpcRouter<R>>,
}

impl<R: TaskRepository> SseTransport<R> {
    pub fn new(router: Arc<JsonRpcRouter<R>>) -> Self {
        Self { router }
    }
    
    /// Handle incoming JSON-RPC request
    pub async fn handle_request(
        &self,
        Json(request): Json<JsonRpcRequest>,
    ) -> Json<JsonRpcResponse> {
        let response = self.router.handle_request(request).await;
        Json(response)
    }
    
    /// Create SSE stream for server-sent events
    pub async fn create_sse_stream(
        &self,
    ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let (tx, rx) = mpsc::channel(100);
        
        // Spawn task to send events
        let router = Arc::clone(&self.router);
        tokio::spawn(async move {
            // Send initial connection event
            let event = Event::default()
                .event("connected")
                .data("MCP v2 Server Connected");
            
            if tx.send(Ok(event)).await.is_err() {
                return;
            }
            
            // Keep connection alive with periodic pings
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                let ping = Event::default()
                    .event("ping")
                    .data(serde_json::json!({
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }).to_string());
                
                if tx.send(Ok(ping)).await.is_err() {
                    break;
                }
            }
        });
        
        let stream = ReceiverStream::new(rx);
        
        Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive"),
        )
    }
    
    /// Send event to SSE clients
    pub async fn send_event(&self, event_type: &str, data: serde_json::Value) {
        // This would be implemented with a broadcast channel to all connected clients
        // For now, this is a placeholder
    }
}

/// Create Axum router for MCP protocol
pub fn create_mcp_router<R: TaskRepository + 'static>(
    repository: Arc<R>,
) -> axum::Router {
    let handler = McpProtocolHandler::new(repository);
    let router = Arc::new(JsonRpcRouter::new(handler));
    let transport = Arc::new(SseTransport::new(router));
    
    axum::Router::new()
        .route("/mcp/v2/rpc", axum::routing::post({
            let transport = Arc::clone(&transport);
            move |req| {
                let transport = Arc::clone(&transport);
                async move { transport.handle_request(req).await }
            }
        }))
        .route("/mcp/v2/sse", axum::routing::get({
            let transport = Arc::clone(&transport);
            move || {
                let transport = Arc::clone(&transport);
                async move { transport.create_sse_stream().await }
            }
        }))
}
```

### 4. Create Protocol Error Handling
In `mcp-protocol/src/error.rs`:

```rust
use core::error::TaskError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    #[error("Internal error: {0}")]
    Internal(#[from] TaskError),
    
    #[error("Transport error: {0}")]
    Transport(String),
}

impl From<serde_json::Error> for ProtocolError {
    fn from(err: serde_json::Error) -> Self {
        ProtocolError::InvalidParams(err.to_string())
    }
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Protocol handler implementation
- `mcp-protocol/src/router.rs` - JSON-RPC request router
- `mcp-protocol/src/transport/sse.rs` - SSE transport
- `mcp-protocol/src/error.rs` - Error types
- `mcp-protocol/src/lib.rs` - Module exports

## Testing Requirements
1. Test all JSON-RPC method handlers
2. Test parameter validation
3. Test error code mapping
4. Test SSE connection and keep-alive
5. Test concurrent request handling
6. Integration tests with mock repository

## Dependencies
```toml
[dependencies]
core = { path = "../core" }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
futures = "0.3"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
tracing = "0.1"
```

## Notes
- SSE transport for real-time updates
- JSON-RPC 2.0 compliance
- All methods async for scalability
- Error codes follow MCP specification
- Keep-alive prevents connection drops