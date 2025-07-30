# PROTOCOL03: Implement Knowledge Objects Handlers

## Objective
Implement all knowledge object-related protocol handlers, enabling agents to create, search, and manage contextual knowledge with full-text search support.

## Implementation Details

### 1. Extend Protocol Handler with Knowledge Methods
In `mcp-protocol/src/handler.rs`, add knowledge handler implementations:

```rust
// Add to the existing McpProtocolHandler implementation
impl<R: TaskRepository> McpProtocolHandler<R> {
    // ... existing methods ...
    
    // ===== Knowledge Object Methods =====
    
    async fn handle_create_knowledge(&self, params: CreateKnowledgeParams) -> Result<KnowledgeObject> {
        // Validate requesting agent matches author
        if params.requesting_agent != params.author_agent_name {
            return Err(TaskError::Validation(
                "Agent can only create knowledge as themselves".to_string()
            ));
        }
        
        // Validate visibility permissions
        self.validate_visibility_permissions(&params)?;
        
        let knowledge = NewKnowledgeObject {
            task_code: params.task_code,
            author_agent_name: params.author_agent_name,
            knowledge_type: params.knowledge_type,
            title: params.title,
            body: params.body,
            tags: params.tags.unwrap_or_default(),
            visibility: params.visibility,
            parent_knowledge_id: params.parent_knowledge_id,
            confidence_score: params.confidence_score,
            artifacts: params.artifacts,
        };
        
        let created = self.repository.create_knowledge_object(knowledge).await?;
        
        // Log event for important knowledge types
        self.log_knowledge_event("knowledge_created", &created).await?;
        
        // Index for search (FTS5 handles this automatically)
        
        Ok(created)
    }
    
    async fn handle_get_knowledge(&self, params: GetKnowledgeParams) -> Result<Vec<KnowledgeObject>> {
        // Build filter with visibility check
        let filter = KnowledgeFilter {
            task_code: params.task_code,
            knowledge_types: params.knowledge_types.unwrap_or_default(),
            author_agent_name: params.author_agent_name,
            visibility: params.visibility,
            tags: params.tags.unwrap_or_default(),
            since: params.since.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            limit: params.limit,
            offset: params.offset,
        };
        
        let mut knowledge_objects = self.repository.get_knowledge_objects(filter).await?;
        
        // Filter by visibility permissions
        knowledge_objects.retain(|ko| {
            self.can_view_knowledge(&params.requesting_agent_name, ko)
        });
        
        Ok(knowledge_objects)
    }
    
    async fn handle_search_knowledge(&self, params: SearchKnowledgeParams) -> Result<Vec<KnowledgeObject>> {
        // Validate search query
        if params.query.trim().is_empty() {
            return Err(TaskError::Validation("Search query cannot be empty".to_string()));
        }
        
        let search_query = KnowledgeSearchQuery {
            query: params.query,
            task_codes: params.task_codes,
            knowledge_types: params.knowledge_types.unwrap_or_default(),
            tags: params.tags.unwrap_or_default(),
            visibility_filter: params.visibility,
            limit: params.limit,
        };
        
        let mut results = self.repository.search_knowledge(search_query).await?;
        
        // Filter by visibility permissions
        results.retain(|ko| {
            self.can_view_knowledge(&params.requesting_agent_name, ko)
        });
        
        // Sort by relevance (would be done by FTS5)
        
        Ok(results)
    }
    
    async fn handle_get_knowledge_tree(&self, params: GetKnowledgeTreeParams) -> Result<Vec<KnowledgeObject>> {
        // Get the tree
        let tree = self.repository.get_knowledge_tree(params.root_knowledge_id).await?;
        
        if tree.is_empty() {
            return Err(TaskError::NotFound(
                format!("Knowledge object {} not found", params.root_knowledge_id)
            ));
        }
        
        // Filter by visibility
        let filtered_tree: Vec<_> = tree.into_iter()
            .filter(|ko| self.can_view_knowledge(&params.requesting_agent_name, ko))
            .collect();
        
        Ok(filtered_tree)
    }
    
    async fn handle_update_knowledge_tags(&self, params: UpdateKnowledgeTagsParams) -> Result<()> {
        // Get knowledge to verify ownership
        let knowledge = self.repository
            .get_knowledge_by_id(params.knowledge_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Knowledge object {} not found", params.knowledge_id)
            ))?;
        
        // Only author can update tags
        if knowledge.author_agent_name != params.requesting_agent {
            return Err(TaskError::Validation(
                "Only the author can update knowledge tags".to_string()
            ));
        }
        
        // Validate tags
        for tag in &params.tags {
            if tag.is_empty() || tag.len() > 50 {
                return Err(TaskError::Validation(
                    "Tags must be between 1 and 50 characters".to_string()
                ));
            }
        }
        
        self.repository.update_knowledge_tags(params.knowledge_id, params.tags).await
    }
    
    async fn handle_get_related_knowledge(&self, params: GetRelatedKnowledgeParams) -> Result<Vec<KnowledgeObject>> {
        let mut related = self.repository
            .get_related_knowledge(params.knowledge_id, params.limit.unwrap_or(10))
            .await?;
        
        // Filter by visibility
        related.retain(|ko| {
            self.can_view_knowledge(&params.requesting_agent_name, ko)
        });
        
        Ok(related)
    }
    
    async fn handle_archive_knowledge(&self, params: ArchiveKnowledgeParams) -> Result<()> {
        // Get knowledge to verify ownership
        let knowledge = self.repository
            .get_knowledge_by_id(params.knowledge_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(
                format!("Knowledge object {} not found", params.knowledge_id)
            ))?;
        
        // Only author or system can archive
        if knowledge.author_agent_name != params.requesting_agent &&
           params.requesting_agent != "system" {
            return Err(TaskError::Validation(
                "Only the author can archive knowledge".to_string()
            ));
        }
        
        self.repository.archive_knowledge(params.knowledge_id).await?;
        
        // Log event
        self.log_knowledge_event("knowledge_archived", &knowledge).await?;
        
        Ok(())
    }
    
    // Helper methods
    
    fn validate_visibility_permissions(&self, params: &CreateKnowledgeParams) -> Result<()> {
        match params.visibility {
            Visibility::Private => {
                // Anyone can create private knowledge
                Ok(())
            }
            Visibility::Team => {
                // Check if agent is part of a team (for now, all agents are on same team)
                Ok(())
            }
            Visibility::Public => {
                // Check if agent has permission to create public knowledge
                // For now, allow all
                Ok(())
            }
        }
    }
    
    fn can_view_knowledge(&self, agent_name: &str, knowledge: &KnowledgeObject) -> bool {
        match knowledge.visibility {
            Visibility::Public => true,
            Visibility::Team => true, // All agents are on same team for now
            Visibility::Private => knowledge.author_agent_name == agent_name,
        }
    }
    
    async fn log_knowledge_event(&self, event_type: &str, knowledge: &KnowledgeObject) -> Result<()> {
        let event = SystemEvent {
            id: 0,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            actor_type: ActorType::Agent,
            actor_id: knowledge.author_agent_name.clone(),
            task_code: Some(knowledge.task_code.clone()),
            payload: serde_json::json!({
                "knowledge_id": knowledge.id,
                "knowledge_type": knowledge.knowledge_type.to_string(),
                "visibility": knowledge.visibility.to_string(),
                "tags": knowledge.tags,
            }),
            correlation_id: Some(format!("knowledge-{}", knowledge.id)),
        };
        
        self.repository.log_event(event).await
    }
}
```

### 2. Add Knowledge-Related JSON-RPC Parameters
In `mcp-protocol/src/params.rs`, add knowledge parameters:

```rust
use core::models::{KnowledgeType, Visibility, KnowledgeObject};

#[derive(Debug, Clone, Deserialize)]
pub struct CreateKnowledgeParams {
    pub task_code: String,
    pub author_agent_name: String,
    pub requesting_agent: String, // For validation
    pub knowledge_type: KnowledgeType,
    pub title: String,
    pub body: String,
    pub tags: Option<Vec<String>>,
    pub visibility: Visibility,
    pub parent_knowledge_id: Option<i32>,
    pub confidence_score: Option<f64>,
    pub artifacts: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetKnowledgeParams {
    pub requesting_agent_name: String,
    pub task_code: Option<String>,
    pub knowledge_types: Option<Vec<KnowledgeType>>,
    pub author_agent_name: Option<String>,
    pub visibility: Option<Visibility>,
    pub tags: Option<Vec<String>>,
    pub since: Option<String>, // ISO 8601
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchKnowledgeParams {
    pub query: String,
    pub requesting_agent_name: String,
    pub task_codes: Option<Vec<String>>,
    pub knowledge_types: Option<Vec<KnowledgeType>>,
    pub tags: Option<Vec<String>>,
    pub visibility: Option<Visibility>,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetKnowledgeTreeParams {
    pub root_knowledge_id: i32,
    pub requesting_agent_name: String,
    pub max_depth: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateKnowledgeTagsParams {
    pub knowledge_id: i32,
    pub tags: Vec<String>,
    pub requesting_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetRelatedKnowledgeParams {
    pub knowledge_id: i32,
    pub requesting_agent_name: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveKnowledgeParams {
    pub knowledge_id: i32,
    pub requesting_agent: String,
    pub reason: Option<String>,
}

// Response types
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchResult {
    pub knowledge: KnowledgeObject,
    pub relevance_score: f64,
    pub snippet: String,
    pub matched_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeTreeResponse {
    pub tree: Vec<KnowledgeTreeNode>,
    pub total_nodes: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeTreeNode {
    pub knowledge: KnowledgeObject,
    pub children: Vec<KnowledgeTreeNode>,
    pub depth: i32,
}
```

### 3. Create Knowledge Search Service
In `mcp-protocol/src/services/knowledge_search.rs`:

```rust
use core::{models::*, repository::TaskRepository};
use std::collections::HashMap;

pub struct KnowledgeSearchService<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> KnowledgeSearchService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Enhanced search with snippet generation
    pub async fn search_with_snippets(
        &self,
        query: KnowledgeSearchQuery,
        requesting_agent: &str,
    ) -> Result<Vec<KnowledgeSearchResult>> {
        // Perform search
        let results = self.repository.search_knowledge(query.clone()).await?;
        
        // Generate snippets and calculate relevance
        let mut enhanced_results = Vec::new();
        
        for knowledge in results {
            // Check visibility
            if !self.can_view_knowledge(requesting_agent, &knowledge) {
                continue;
            }
            
            // Generate snippet
            let snippet = self.generate_snippet(&knowledge.body, &query.query, 150);
            
            // Calculate relevance score
            let relevance_score = self.calculate_relevance(&knowledge, &query.query);
            
            // Determine matched fields
            let matched_fields = self.get_matched_fields(&knowledge, &query.query);
            
            enhanced_results.push(KnowledgeSearchResult {
                knowledge,
                relevance_score,
                snippet,
                matched_fields,
            });
        }
        
        // Sort by relevance
        enhanced_results.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score).unwrap()
        });
        
        Ok(enhanced_results)
    }
    
    /// Find similar knowledge objects
    pub async fn find_similar(
        &self,
        knowledge_id: i32,
        limit: i32,
    ) -> Result<Vec<KnowledgeObject>> {
        // Get source knowledge
        let source = self.repository
            .get_knowledge_by_id(knowledge_id)
            .await?
            .ok_or_else(|| TaskError::NotFound(format!("Knowledge {} not found", knowledge_id)))?;
        
        // Search using title and tags
        let query = KnowledgeSearchQuery {
            query: source.title.clone(),
            task_codes: Some(vec![source.task_code.clone()]),
            knowledge_types: vec![source.knowledge_type.clone()],
            tags: source.tags.clone(),
            visibility_filter: None,
            limit: Some(limit + 1), // +1 to exclude self
        };
        
        let mut results = self.repository.search_knowledge(query).await?;
        
        // Remove self from results
        results.retain(|k| k.id != knowledge_id);
        results.truncate(limit as usize);
        
        Ok(results)
    }
    
    /// Build knowledge graph connections
    pub async fn get_knowledge_graph(
        &self,
        task_code: &str,
        requesting_agent: &str,
    ) -> Result<KnowledgeGraph> {
        let filter = KnowledgeFilter {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        };
        
        let all_knowledge = self.repository.get_knowledge_objects(filter).await?;
        
        // Build graph structure
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        
        for knowledge in all_knowledge {
            if !self.can_view_knowledge(requesting_agent, &knowledge) {
                continue;
            }
            
            nodes.push(KnowledgeNode {
                id: knowledge.id,
                title: knowledge.title.clone(),
                knowledge_type: knowledge.knowledge_type.clone(),
                author: knowledge.author_agent_name.clone(),
            });
            
            // Add parent-child edges
            if let Some(parent_id) = knowledge.parent_knowledge_id {
                edges.push(KnowledgeEdge {
                    from: parent_id,
                    to: knowledge.id,
                    edge_type: "parent_child".to_string(),
                });
            }
            
            // Add tag-based connections
            // Would need to compare tags with other knowledge objects
        }
        
        Ok(KnowledgeGraph { nodes, edges })
    }
    
    // Helper methods
    
    fn generate_snippet(&self, body: &str, query: &str, max_length: usize) -> String {
        // Find query terms in body
        let query_lower = query.to_lowercase();
        let body_lower = body.to_lowercase();
        
        if let Some(pos) = body_lower.find(&query_lower) {
            // Extract context around match
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 100).min(body.len());
            
            let mut snippet = String::new();
            if start > 0 {
                snippet.push_str("...");
            }
            snippet.push_str(&body[start..end]);
            if end < body.len() {
                snippet.push_str("...");
            }
            
            snippet
        } else {
            // No direct match, return beginning
            let end = max_length.min(body.len());
            if body.len() > max_length {
                format!("{}...", &body[..end])
            } else {
                body.to_string()
            }
        }
    }
    
    fn calculate_relevance(&self, knowledge: &KnowledgeObject, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let mut score = 0.0;
        
        // Title match (highest weight)
        if knowledge.title.to_lowercase().contains(&query_lower) {
            score += 0.5;
        }
        
        // Body match
        let body_matches = knowledge.body.to_lowercase()
            .matches(&query_lower).count();
        score += (body_matches as f64 * 0.1).min(0.3);
        
        // Tag match
        for tag in &knowledge.tags {
            if tag.to_lowercase().contains(&query_lower) {
                score += 0.1;
            }
        }
        
        // Confidence score bonus
        if let Some(confidence) = knowledge.confidence_score {
            score += confidence * 0.1;
        }
        
        score.min(1.0)
    }
    
    fn get_matched_fields(&self, knowledge: &KnowledgeObject, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut fields = Vec::new();
        
        if knowledge.title.to_lowercase().contains(&query_lower) {
            fields.push("title".to_string());
        }
        
        if knowledge.body.to_lowercase().contains(&query_lower) {
            fields.push("body".to_string());
        }
        
        for tag in &knowledge.tags {
            if tag.to_lowercase().contains(&query_lower) {
                fields.push("tags".to_string());
                break;
            }
        }
        
        fields
    }
    
    fn can_view_knowledge(&self, agent_name: &str, knowledge: &KnowledgeObject) -> bool {
        match knowledge.visibility {
            Visibility::Public => true,
            Visibility::Team => true,
            Visibility::Private => knowledge.author_agent_name == agent_name,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<KnowledgeNode>,
    pub edges: Vec<KnowledgeEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeNode {
    pub id: i32,
    pub title: String,
    pub knowledge_type: KnowledgeType,
    pub author: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeEdge {
    pub from: i32,
    pub to: i32,
    pub edge_type: String,
}
```

### 4. Create Knowledge Export Service
In `mcp-protocol/src/services/knowledge_export.rs`:

```rust
use std::io::Write;

pub struct KnowledgeExportService<R: TaskRepository> {
    repository: Arc<R>,
}

impl<R: TaskRepository> KnowledgeExportService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
    
    /// Export knowledge as Markdown
    pub async fn export_as_markdown(
        &self,
        task_code: &str,
        requesting_agent: &str,
    ) -> Result<String> {
        let filter = KnowledgeFilter {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        };
        
        let knowledge_objects = self.repository.get_knowledge_objects(filter).await?;
        
        let mut output = String::new();
        output.push_str(&format!("# Knowledge Base for Task {}\n\n", task_code));
        
        // Group by type
        let mut by_type: HashMap<KnowledgeType, Vec<&KnowledgeObject>> = HashMap::new();
        
        for ko in &knowledge_objects {
            if self.can_view_knowledge(requesting_agent, ko) {
                by_type.entry(ko.knowledge_type.clone())
                    .or_default()
                    .push(ko);
            }
        }
        
        // Write each type section
        for (knowledge_type, items) in by_type {
            output.push_str(&format!("## {}\n\n", knowledge_type));
            
            for item in items {
                output.push_str(&format!("### {}\n", item.title));
                output.push_str(&format!("*By {} on {}*\n\n", 
                    item.author_agent_name, 
                    item.created_at.format("%Y-%m-%d %H:%M UTC")
                ));
                
                if !item.tags.is_empty() {
                    output.push_str(&format!("Tags: {}\n\n", item.tags.join(", ")));
                }
                
                output.push_str(&item.body);
                output.push_str("\n\n---\n\n");
            }
        }
        
        Ok(output)
    }
    
    /// Export knowledge as JSON
    pub async fn export_as_json(
        &self,
        task_code: &str,
        requesting_agent: &str,
    ) -> Result<serde_json::Value> {
        let filter = KnowledgeFilter {
            task_code: Some(task_code.to_string()),
            ..Default::default()
        };
        
        let mut knowledge_objects = self.repository.get_knowledge_objects(filter).await?;
        
        // Filter by visibility
        knowledge_objects.retain(|ko| self.can_view_knowledge(requesting_agent, ko));
        
        Ok(serde_json::json!({
            "task_code": task_code,
            "export_date": Utc::now().to_rfc3339(),
            "exported_by": requesting_agent,
            "knowledge_objects": knowledge_objects,
            "total_count": knowledge_objects.len(),
        }))
    }
    
    fn can_view_knowledge(&self, agent_name: &str, knowledge: &KnowledgeObject) -> bool {
        match knowledge.visibility {
            Visibility::Public => true,
            Visibility::Team => true,
            Visibility::Private => knowledge.author_agent_name == agent_name,
        }
    }
}
```

## Files to Create/Modify
- `mcp-protocol/src/handler.rs` - Add knowledge handler methods
- `mcp-protocol/src/params.rs` - Add knowledge parameter types
- `mcp-protocol/src/services/knowledge_search.rs` - Knowledge search service
- `mcp-protocol/src/services/knowledge_export.rs` - Knowledge export service
- `mcp-protocol/src/router.rs` - Add knowledge method routing

## Testing Requirements
1. Test knowledge creation with all visibility levels
2. Test parent-child relationships
3. Test full-text search with various queries
4. Test tag filtering and updates
5. Test visibility enforcement
6. Test knowledge export formats
7. Test concurrent knowledge operations

## Notes
- FTS5 provides full-text search capabilities
- Visibility rules strictly enforced
- Knowledge graphs for relationship visualization
- Export functionality for documentation
- Confidence scores for decision tracking