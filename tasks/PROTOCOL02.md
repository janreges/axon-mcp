# PROTOCOL02: Implement Task Messages Handlers

## Objective
Implement all task message-related protocol handlers, enabling agent-to-agent communication through the MCP protocol with proper validation and threading support.

## Implementation Details

### 1. Extend Protocol Handler with Message Methods
In `mcp-protocol/src/handler.rs`, add message handler implementations:

```rust
// Add to the existing McpProtocolHandler implementation
impl<R: TaskRepository> McpProtocolHandler<R> {
    // ... existing methods ...
    
    // ===== Task Message Methods =====
    
    async fn handle_add_task_message(&self, params: AddTaskMessageParams) -> Result<TaskMessage> {
        // Validate requesting agent matches author
        if params.requesting_agent != params.author_agent_name {
            return Err(TaskError::Validation(
                "Agent can only create messages as themselves".to_string()
            ));
        }
        
        // Validate message type is appropriate
        match params.message_type {
            MessageType::Handoff => {
                // Handoff messages should use the handoff system
                return Err(TaskError::Validation(
                    "Use handoff/create for handoff messages".to_string()
                ));
            }
            _ => {}
        }
        
        let message = NewTaskMessage {
            task_code: params.task_code,
            author_agent_name: params.author_agent_name,
            message_type: params.message_type,
            content: params.content,
            reply_to_message_id: params.reply_to_message_id,
        };
        
        let created_message = self.repository.add_task_message(message).await?;
        
        // Log event for important message types
        match created_message.message_type {
            MessageType::Blocker => {
                self.log_message_event("blocker_reported", &created_message).await?;
            }
            MessageType::Solution => {
                self.log_message_event("solution_provided", &created_message).await?;
            }
            MessageType::Review => {
                self.log_message_event("review_requested", &created_message).await?;
            }
            _ => {}
        }
        
        Ok(created_message)
    }
    
    async fn handle_get_task_messages(&self, params: GetTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        // Build filter
        let filter = MessageFilter {
            task_code: Some(params.task_code),
            message_types: params.message_types.unwrap_or_default(),
            since: params.since.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            author_agent_name: params.author_agent_name,
            limit: params.limit,
            offset: params.offset,
        };
        
        let messages = self.repository.get_task_messages(filter).await?;
        
        // Mark messages as read if requested
        if params.mark_as_read.unwrap_or(false) {
            self.repository.mark_messages_read(
                &params.task_code,
                &params.requesting_agent_name,
            ).await?;
        }
        
        Ok(messages)
    }
    
    async fn handle_search_task_messages(&self, params: SearchTaskMessagesParams) -> Result<Vec<TaskMessage>> {
        // Validate search query
        if params.query.trim().is_empty() {
            return Err(TaskError::Validation("Search query cannot be empty".to_string()));
        }
        
        if params.query.len() < 3 {
            return Err(TaskError::Validation("Search query must be at least 3 characters".to_string()));
        }
        
        let search_query = MessageSearchQuery {
            query: params.query,
            task_codes: params.task_codes,
            message_types: params.message_types.unwrap_or_default(),
            limit: params.limit,
        };
        
        self.repository.search_task_messages(search_query).await
    }
    
    async fn handle_get_message_thread(&self, params: GetMessageThreadParams) -> Result<Vec<TaskMessage>> {
        // Get the thread
        let thread = self.repository.get_message_thread(params.message_id).await?;
        
        if thread.is_empty() {
            return Err(TaskError::NotFound(format!("Message {} not found", params.message_id)));
        }
        
        // Optionally filter by depth
        if let Some(max_depth) = params.max_depth {
            // This would require depth information from the query
            // For now, return full thread
        }
        
        Ok(thread)
    }
    
    async fn handle_get_message_stats(&self, params: GetMessageStatsParams) -> Result<MessageCountByType> {
        self.repository.count_task_messages(&params.task_code).await
    }
    
    async fn handle_get_unread_count(&self, params: GetUnreadCountParams) -> Result<i32> {
        self.repository.get_unread_count(&params.agent_name).await
    }
    
    // Helper method to log message events
    async fn log_message_event(&self, event_type: &str, message: &TaskMessage) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            actor_type: ActorType::Agent,
            actor_id: message.author_agent_name.clone(),
            task_code: Some(message.task_code.clone()),
            payload: serde_json::json!({
                "message_id": message.id,
                "message_type": message.message_type.to_string(),
                "reply_to": message.reply_to_message_id,
            }),
            correlation_id: Some(format!("msg-{}", message.id)),
        };
        
        self.repository.log_event(event).await
    }
}
```

### 2. Add Message-Related JSON-RPC Parameters
In `mcp-protocol/src/params.rs`:

```rust
use core::models::{MessageType, TaskMessage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct AddTaskMessageParams {
    pub task_code: String,
    pub author_agent_name: String,
    pub requesting_agent: String, // For validation
    pub message_type: MessageType,
    pub content: String,
    pub reply_to_message_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetTaskMessagesParams {
    pub task_code: String,
    pub requesting_agent_name: String,
    pub message_types: Option<Vec<MessageType>>,
    pub since: Option<String>, // ISO 8601 datetime
    pub author_agent_name: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub mark_as_read: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchTaskMessagesParams {
    pub query: String,
    pub requesting_agent_name: String,
    pub task_codes: Option<Vec<String>>,
    pub message_types: Option<Vec<MessageType>>,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetMessageThreadParams {
    pub message_id: i32,
    pub requesting_agent_name: String,
    pub max_depth: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetMessageStatsParams {
    pub task_code: String,
    pub requesting_agent_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetUnreadCountParams {
    pub agent_name: String,
}

// Response types
#[derive(Debug, Clone, Serialize)]
pub struct MessageThreadResponse {
    pub messages: Vec<TaskMessage>,
    pub total_depth: i32,
    pub root_message_id: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnreadCountResponse {
    pub agent_name: String,
    pub unread_count: i32,
    pub by_task: Vec<TaskUnreadCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskUnreadCount {
    pub task_code: String,
    pub unread_count: i32,
}
```

### 3. Add Message Router Handlers
In `mcp-protocol/src/router.rs`, add message method routing:

```rust
impl<R: TaskRepository> JsonRpcRouter<R> {
    // ... existing methods ...
    
    async fn handle_add_message(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let add_params: AddTaskMessageParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_add_task_message(add_params).await {
            Ok(message) => Ok(serde_json::to_value(message).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_get_messages(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let get_params: GetTaskMessagesParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_get_task_messages(get_params).await {
            Ok(messages) => Ok(serde_json::to_value(messages).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_search_messages(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let search_params: SearchTaskMessagesParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_search_task_messages(search_params).await {
            Ok(messages) => Ok(serde_json::to_value(messages).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_get_message_thread(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let thread_params: GetMessageThreadParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_get_message_thread(thread_params).await {
            Ok(messages) => {
                // Build thread response
                let root_id = messages.first()
                    .and_then(|m| if m.reply_to_message_id.is_none() { Some(m.id) } else { None })
                    .unwrap_or(0);
                
                let response = MessageThreadResponse {
                    messages,
                    total_depth: 0, // Would need to calculate
                    root_message_id: root_id,
                };
                
                Ok(serde_json::to_value(response).unwrap())
            }
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_get_message_stats(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let stats_params: GetMessageStatsParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_get_message_stats(stats_params).await {
            Ok(stats) => Ok(serde_json::to_value(stats).unwrap()),
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
    
    async fn handle_get_unread_count(&self, params: Option<Value>) -> Result<Value, JsonRpcError> {
        let params = params
            .ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        
        let unread_params: GetUnreadCountParams = serde_json::from_value(params)
            .map_err(|e| JsonRpcError::invalid_params(&e.to_string()))?;
        
        match self.handler.handle_get_unread_count(unread_params.clone()).await {
            Ok(count) => {
                let response = UnreadCountResponse {
                    agent_name: unread_params.agent_name,
                    unread_count: count,
                    by_task: vec![], // Would need separate query
                };
                
                Ok(serde_json::to_value(response).unwrap())
            }
            Err(e) => Err(JsonRpcError::internal_error(&e)),
        }
    }
}
```

### 4. Create Message Notification System
In `mcp-protocol/src/notifications/messages.rs`:

```rust
use crate::transport::sse::SseTransport;
use core::{models::*, repository::TaskRepository};
use tokio::sync::broadcast;

pub struct MessageNotificationService<R: TaskRepository> {
    repository: Arc<R>,
    event_sender: broadcast::Sender<MessageEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageEvent {
    pub event_type: String,
    pub task_code: String,
    pub message: TaskMessage,
    pub mentioned_agents: Vec<String>,
}

impl<R: TaskRepository> MessageNotificationService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            repository,
            event_sender: tx,
        }
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<MessageEvent> {
        self.event_sender.subscribe()
    }
    
    pub async fn notify_new_message(&self, message: &TaskMessage) {
        // Extract mentioned agents from content
        let mentioned = self.extract_mentions(&message.content);
        
        // Get task participants
        let participants = self.get_task_participants(&message.task_code).await;
        
        // Create event
        let event = MessageEvent {
            event_type: "new_message".to_string(),
            task_code: message.task_code.clone(),
            message: message.clone(),
            mentioned_agents: mentioned.clone(),
        };
        
        // Broadcast event
        let _ = self.event_sender.send(event);
        
        // Special handling for certain message types
        match message.message_type {
            MessageType::Question => {
                self.notify_question_asked(message, &participants).await;
            }
            MessageType::Blocker => {
                self.notify_blocker_reported(message, &participants).await;
            }
            MessageType::Solution => {
                self.notify_solution_provided(message).await;
            }
            _ => {}
        }
    }
    
    fn extract_mentions(&self, content: &str) -> Vec<String> {
        // Extract @agent-name mentions
        let mention_regex = regex::Regex::new(r"@([a-z0-9-]+)").unwrap();
        mention_regex
            .captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect()
    }
    
    async fn get_task_participants(&self, task_code: &str) -> Vec<String> {
        // Get all agents who have sent messages on this task
        let filter = MessageFilter {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        };
        
        if let Ok(messages) = self.repository.get_task_messages(filter).await {
            let mut participants = HashSet::new();
            for msg in messages {
                participants.insert(msg.author_agent_name);
            }
            participants.into_iter().collect()
        } else {
            vec![]
        }
    }
    
    async fn notify_question_asked(&self, message: &TaskMessage, participants: &[String]) {
        // Could send specific notifications to agents with relevant capabilities
        let event = MessageEvent {
            event_type: "question_needs_answer".to_string(),
            task_code: message.task_code.clone(),
            message: message.clone(),
            mentioned_agents: vec![],
        };
        
        let _ = self.event_sender.send(event);
    }
    
    async fn notify_blocker_reported(&self, message: &TaskMessage, participants: &[String]) {
        // High priority notification
        let event = MessageEvent {
            event_type: "blocker_reported".to_string(),
            task_code: message.task_code.clone(),
            message: message.clone(),
            mentioned_agents: vec![],
        };
        
        let _ = self.event_sender.send(event);
    }
    
    async fn notify_solution_provided(&self, message: &TaskMessage) {
        // Notify that a solution was provided
        if let Some(reply_to) = message.reply_to_message_id {
            // Get the original message to notify the author
            if let Ok(Some(original)) = self.repository.get_message_by_id(reply_to).await {
                let event = MessageEvent {
                    event_type: "solution_received".to_string(),
                    task_code: message.task_code.clone(),
                    message: message.clone(),
                    mentioned_agents: vec![original.author_agent_name],
                };
                
                let _ = self.event_sender.send(event);
            }
        }
    }
}
```

### 5. Create Message Validation Module
In `mcp-protocol/src/validation/messages.rs`:

```rust
use core::{error::*, models::*};

pub struct MessageValidator;

impl MessageValidator {
    /// Validate message content based on type
    pub fn validate_message_content(
        message_type: &MessageType,
        content: &str,
    ) -> Result<()> {
        // Check content length
        if content.is_empty() {
            return Err(TaskError::Validation("Message content cannot be empty".to_string()));
        }
        
        if content.len() > 10000 {
            return Err(TaskError::Validation(
                "Message content cannot exceed 10000 characters".to_string()
            ));
        }
        
        // Type-specific validation
        match message_type {
            MessageType::Question => {
                if !content.contains('?') {
                    return Err(TaskError::Validation(
                        "Question messages should contain a question mark".to_string()
                    ));
                }
            }
            MessageType::Blocker => {
                if content.len() < 20 {
                    return Err(TaskError::Validation(
                        "Blocker descriptions should be at least 20 characters".to_string()
                    ));
                }
            }
            MessageType::Solution => {
                if content.len() < 10 {
                    return Err(TaskError::Validation(
                        "Solution descriptions should be at least 10 characters".to_string()
                    ));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Validate reply context
    pub fn validate_reply_context(
        message_type: &MessageType,
        reply_to_type: Option<&MessageType>,
    ) -> Result<()> {
        match (message_type, reply_to_type) {
            (MessageType::Solution, Some(MessageType::Question)) => Ok(()),
            (MessageType::Solution, Some(MessageType::Blocker)) => Ok(()),
            (MessageType::Solution, Some(_)) => {
                Err(TaskError::Validation(
                    "Solutions should reply to questions or blockers".to_string()
                ))
            }
            _ => Ok(()),
        }
    }
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Add message handler methods
- `mcp-protocol/src/params.rs` - New file for parameter types
- `mcp-protocol/src/router.rs` - Add message routing
- `mcp-protocol/src/notifications/messages.rs` - Message notification service
- `mcp-protocol/src/validation/messages.rs` - Message validation

## Testing Requirements
1. Test message creation with validation
2. Test reply threading
3. Test message filtering and pagination
4. Test search functionality
5. Test notification system
6. Test mention extraction
7. Test concurrent message handling

## Dependencies
```toml
[dependencies]
regex = "1.10"
```

## Notes
- Message notifications via SSE
- Thread safety for concurrent messages
- Mention system for direct notifications
- Type-specific validation rules
- Event logging for audit trail