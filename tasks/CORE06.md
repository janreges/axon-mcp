# CORE06: Extend TaskRepository Trait - Knowledge

## Objective
Extend the TaskRepository trait in the core crate to include methods for managing knowledge objects, enabling agents to store and retrieve contextual information and decisions.

## Current State
The TaskRepository trait needs knowledge management methods to support the MCP v2 knowledge preservation system.

## Required Changes

### 1. Add Knowledge-Related Types to Core
Ensure these types exist in `core/src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeObject {
    pub id: i32,
    pub task_code: String,
    pub author_agent_name: String,
    pub knowledge_type: KnowledgeType,
    pub created_at: DateTime<Utc>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub parent_knowledge_id: Option<i32>,
    pub confidence_score: Option<f64>,
    pub artifacts: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKnowledgeObject {
    pub task_code: String,
    pub author_agent_name: String,
    pub knowledge_type: KnowledgeType,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub parent_knowledge_id: Option<i32>,
    pub confidence_score: Option<f64>,
    pub artifacts: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeFilter {
    pub task_code: Option<String>,
    pub knowledge_types: Vec<KnowledgeType>,
    pub author_agent_name: Option<String>,
    pub visibility: Option<Visibility>,
    pub tags: Vec<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSearchQuery {
    pub query: String,
    pub task_codes: Option<Vec<String>>,
    pub knowledge_types: Vec<KnowledgeType>,
    pub tags: Vec<String>,
    pub visibility_filter: Option<Visibility>,
    pub limit: Option<i32>,
}
```

### 2. Extend TaskRepository Trait
Add to `core/src/repository.rs`:

```rust
#[async_trait]
pub trait TaskRepository: Send + Sync {
    // ... existing methods ...
    
    // ===== Knowledge Objects Methods =====
    
    /// Create a new knowledge object
    async fn create_knowledge_object(&self, knowledge: NewKnowledgeObject) -> Result<KnowledgeObject>;
    
    /// Get knowledge objects with filtering
    async fn get_knowledge_objects(&self, filter: KnowledgeFilter) -> Result<Vec<KnowledgeObject>>;
    
    /// Get a specific knowledge object by ID
    async fn get_knowledge_by_id(&self, knowledge_id: i32) -> Result<Option<KnowledgeObject>>;
    
    /// Search knowledge objects using full-text search
    async fn search_knowledge(&self, query: KnowledgeSearchQuery) -> Result<Vec<KnowledgeObject>>;
    
    /// Get knowledge hierarchy (object and all children)
    async fn get_knowledge_tree(&self, root_id: i32) -> Result<Vec<KnowledgeObject>>;
    
    /// Update knowledge object tags
    async fn update_knowledge_tags(&self, knowledge_id: i32, tags: Vec<String>) -> Result<()>;
    
    /// Get related knowledge objects
    async fn get_related_knowledge(&self, knowledge_id: i32, limit: i32) -> Result<Vec<KnowledgeObject>>;
    
    /// Count knowledge objects by type for a task
    async fn count_knowledge_by_type(&self, task_code: &str) -> Result<KnowledgeCountByType>;
    
    /// Archive knowledge objects (soft delete)
    async fn archive_knowledge(&self, knowledge_id: i32) -> Result<()>;
}
```

### 3. Add Supporting Types
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCountByType {
    pub total: i32,
    pub by_type: HashMap<KnowledgeType, i32>,
    pub public_count: i32,
    pub team_count: i32,
    pub private_count: i32,
}

impl Default for KnowledgeCountByType {
    fn default() -> Self {
        Self {
            total: 0,
            by_type: HashMap::new(),
            public_count: 0,
            team_count: 0,
            private_count: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTreeNode {
    pub knowledge: KnowledgeObject,
    pub children: Vec<KnowledgeTreeNode>,
}
```

### 4. Add Validation Methods
```rust
impl NewKnowledgeObject {
    pub fn validate(&self) -> Result<()> {
        // Validate task code
        if self.task_code.is_empty() {
            return Err(TaskError::Validation("Task code cannot be empty".to_string()));
        }
        
        // Validate agent name format
        if !self.author_agent_name.chars().all(|c| c.is_lowercase() || c == '-' || c.is_numeric()) {
            return Err(TaskError::Validation(
                "Agent name must be in kebab-case format".to_string()
            ));
        }
        
        // Validate title length
        if self.title.is_empty() || self.title.len() > 200 {
            return Err(TaskError::Validation(
                "Title must be between 1 and 200 characters".to_string()
            ));
        }
        
        // Validate body length
        if self.body.is_empty() || self.body.len() > 50000 {
            return Err(TaskError::Validation(
                "Body must be between 1 and 50000 characters".to_string()
            ));
        }
        
        // Validate confidence score if provided
        if let Some(score) = self.confidence_score {
            if !(0.0..=1.0).contains(&score) {
                return Err(TaskError::Validation(
                    "Confidence score must be between 0.0 and 1.0".to_string()
                ));
            }
        }
        
        // Validate tags
        if self.tags.len() > 20 {
            return Err(TaskError::Validation(
                "Cannot have more than 20 tags".to_string()
            ));
        }
        
        Ok(())
    }
}
```

### 5. Add Helper Methods to KnowledgeFilter
```rust
impl KnowledgeFilter {
    /// Create a filter for a specific task
    pub fn for_task(task_code: &str) -> Self {
        Self {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        }
    }
    
    /// Create a filter for specific knowledge types
    pub fn by_types(types: Vec<KnowledgeType>) -> Self {
        Self {
            knowledge_types: types,
            ..Default::default()
        }
    }
    
    /// Filter for public knowledge only
    pub fn public_only() -> Self {
        Self {
            visibility: Some(Visibility::Public),
            ..Default::default()
        }
    }
    
    /// Filter by tags (any match)
    pub fn with_tags(tags: Vec<String>) -> Self {
        Self {
            tags,
            ..Default::default()
        }
    }
    
    /// Add pagination
    pub fn with_pagination(mut self, limit: i32, offset: i32) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
}
```

### 6. Protocol Handler Trait Extension
Add to `ProtocolHandler` trait:

```rust
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    // ... existing methods ...
    
    // Knowledge Objects
    async fn create_knowledge(&self, params: CreateKnowledgeParams) -> Result<KnowledgeObject>;
    async fn get_knowledge(&self, params: GetKnowledgeParams) -> Result<Vec<KnowledgeObject>>;
    async fn search_knowledge(&self, params: SearchKnowledgeParams) -> Result<Vec<KnowledgeObject>>;
    async fn get_knowledge_tree(&self, params: GetKnowledgeTreeParams) -> Result<Vec<KnowledgeTreeNode>>;
}

// Parameter types
#[derive(Debug, Deserialize)]
pub struct CreateKnowledgeParams {
    pub task_code: String,
    pub author_agent_name: String,
    pub knowledge_type: KnowledgeType,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub visibility: Visibility,
    pub parent_knowledge_id: Option<i32>,
    pub confidence_score: Option<f64>,
    pub artifacts: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct GetKnowledgeParams {
    pub task_code: Option<String>,
    pub requesting_agent_name: String,
    pub knowledge_types: Vec<KnowledgeType>,
    pub visibility: Option<Visibility>,
    pub tags: Vec<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchKnowledgeParams {
    pub query: String,
    pub requesting_agent_name: String,
    pub task_codes: Option<Vec<String>>,
    pub knowledge_types: Vec<KnowledgeType>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetKnowledgeTreeParams {
    pub root_knowledge_id: i32,
    pub requesting_agent_name: String,
}
```

## Files to Modify
- `core/src/repository.rs` - Add knowledge methods to trait
- `core/src/protocol.rs` - Add protocol handler methods
- `core/src/models.rs` - Add knowledge-related types
- `core/src/error.rs` - Add any new error variants if needed

## Testing Requirements
1. Mock implementations for all new methods
2. Unit tests for validation methods
3. Tests for filter builders
4. Tests for visibility rules
5. Integration tests will be in database crate

## Notes
- Knowledge objects are immutable once created
- Tags can be updated separately for flexibility
- Full-text search will use FTS5 in SQLite
- Parent-child relationships enable knowledge trees
- Visibility rules must be enforced at query level