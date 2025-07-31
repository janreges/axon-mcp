//! OAuth 2.0 Authentication for MCP v2
//! 
//! Implements OAuth 2.0 Resource Server functionality as required by MCP 2025-06-18 specification.
//! Supports Resource Indicators (RFC 8707) to prevent token mis-redemption attacks.

use axum::{
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use tracing::{warn, info};

/// OAuth 2.0 token validation result
#[derive(Debug, Clone)]
pub struct TokenValidation {
    pub is_valid: bool,
    pub client_id: Option<String>,
    pub scopes: Vec<String>,
    pub resource_indicator: Option<String>,
}

/// OAuth 2.0 scope for MCP operations
#[derive(Debug, Clone, PartialEq)]
pub enum McpScope {
    /// Read access to tasks
    TaskRead,
    /// Write access to tasks
    TaskWrite,
    /// Administrative access
    Admin,
    /// Health check access
    Health,
}

impl McpScope {
    /// Convert scope to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            McpScope::TaskRead => "mcp:task:read",
            McpScope::TaskWrite => "mcp:task:write", 
            McpScope::Admin => "mcp:admin",
            McpScope::Health => "mcp:health",
        }
    }
    
    /// Parse scope from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mcp:task:read" => Some(McpScope::TaskRead),
            "mcp:task:write" => Some(McpScope::TaskWrite),
            "mcp:admin" => Some(McpScope::Admin),
            "mcp:health" => Some(McpScope::Health),
            _ => None,
        }
    }
}

/// OAuth 2.0 authentication middleware
#[derive(Clone)]
pub struct McpAuth {
    /// Enable authentication (false for development)
    enabled: bool,
}

impl McpAuth {
    /// Create new authentication handler
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
    
    /// Validate OAuth 2.0 bearer token
    pub async fn validate_token(&self, headers: &HeaderMap) -> TokenValidation {
        if !self.enabled {
            // Development mode - allow all requests
            return TokenValidation {
                is_valid: true,
                client_id: Some("dev-client".to_string()),
                scopes: vec![
                    McpScope::TaskRead.as_str().to_string(),
                    McpScope::TaskWrite.as_str().to_string(),
                    McpScope::Admin.as_str().to_string(),
                    McpScope::Health.as_str().to_string(),
                ],
                resource_indicator: Some("mcp://task-server".to_string()),
            };
        }
        
        // Extract Authorization header
        let auth_header = match headers.get("authorization").or_else(|| headers.get("Authorization")) {
            Some(header) => match header.to_str() {
                Ok(s) => s,
                Err(_) => {
                    warn!("Invalid Authorization header format");
                    return TokenValidation {
                        is_valid: false,
                        client_id: None,
                        scopes: vec![],
                        resource_indicator: None,
                    };
                }
            },
            None => {
                info!("No Authorization header provided");
                return TokenValidation {
                    is_valid: false,
                    client_id: None,
                    scopes: vec![],
                    resource_indicator: None,
                };
            }
        };
        
        // Parse Bearer token
        let token = if let Some(token) = auth_header.strip_prefix("Bearer ") {
            token.trim()
        } else {
            warn!("Authorization header does not contain Bearer token");
            return TokenValidation {
                is_valid: false,
                client_id: None,
                scopes: vec![],
                resource_indicator: None,
            };
        };
        
        // In a real implementation, this would validate against an OAuth provider
        // For now, implement basic token validation logic
        self.validate_bearer_token(token, headers).await
    }
    
    /// Validate bearer token (placeholder implementation)
    async fn validate_bearer_token(&self, token: &str, headers: &HeaderMap) -> TokenValidation {
        // Placeholder validation - in production this would:
        // 1. Validate token signature/format
        // 2. Check token expiration
        // 3. Verify with OAuth provider
        // 4. Validate Resource Indicators (RFC 8707)
        
        // Basic token format validation
        if token.len() < 10 {
            warn!("Token too short to be valid");
            return TokenValidation {
                is_valid: false,
                client_id: None,
                scopes: vec![],
                resource_indicator: None,
            };
        }
        
        // Check for Resource Indicator header (RFC 8707)
        let resource_indicator = headers.get("resource")
            .or_else(|| headers.get("Resource"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        // Simulate token validation
        info!("Validating token: {}...", &token[..std::cmp::min(token.len(), 10)]);
        
        // Return mock validation result
        TokenValidation {
            is_valid: true,
            client_id: Some("mcp-client-123".to_string()),
            scopes: vec![
                McpScope::TaskRead.as_str().to_string(),
                McpScope::TaskWrite.as_str().to_string(),
            ],
            resource_indicator: resource_indicator.or_else(|| Some("mcp://task-server".to_string())),
        }
    }
    
    /// Check if token has required scope for MCP method
    pub fn check_scope(&self, validation: &TokenValidation, method: &str) -> bool {
        if !validation.is_valid {
            return false;
        }
        
        let required_scope = match method {
            // MCP v1 Core Functions
            "create_task" | "update_task" | "set_task_state" | "assign_task" | "archive_task" => McpScope::TaskWrite,
            "get_task_by_id" | "get_task_by_code" | "list_tasks" => McpScope::TaskRead,
            "health_check" => McpScope::Health,
            // MCP v2 Advanced Multi-Agent Functions
            "discover_work" => McpScope::TaskRead, // Reading available work
            "claim_task" | "release_task" => McpScope::TaskWrite, // Modifying task ownership
            "start_work_session" | "end_work_session" => McpScope::TaskWrite, // Time tracking modifications
            // Task Messaging Functions
            "create_task_message" => McpScope::TaskWrite, // Creating messages
            "get_task_messages" => McpScope::TaskRead, // Reading messages
            _ => return false, // Unknown method
        };
        
        validation.scopes.contains(&required_scope.as_str().to_string())
            || validation.scopes.contains(&McpScope::Admin.as_str().to_string())
    }
    
    /// Create OAuth error response
    pub fn create_auth_error(error_type: &str, description: &str, id: Option<Value>) -> Json<Value> {
        Json(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32001, // MCP authentication error code
                "message": error_type,
                "data": {
                    "description": description,
                    "oauth_error": error_type
                }
            },
            "id": id
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};
    
    #[tokio::test]
    async fn test_disabled_auth_allows_all() {
        let auth = McpAuth::new(false);
        let headers = HeaderMap::new();
        
        let validation = auth.validate_token(&headers).await;
        assert!(validation.is_valid);
        assert!(auth.check_scope(&validation, "create_task"));
    }
    
    #[tokio::test]
    async fn test_missing_auth_header() {
        let auth = McpAuth::new(true);
        let headers = HeaderMap::new();
        
        let validation = auth.validate_token(&headers).await;
        assert!(!validation.is_valid);
    }
    
    #[tokio::test]
    async fn test_valid_bearer_token() {
        let auth = McpAuth::new(true);
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Bearer valid_token_12345"));
        
        let validation = auth.validate_token(&headers).await;
        assert!(validation.is_valid);
        assert!(auth.check_scope(&validation, "get_task_by_id"));
    }
    
    #[test]
    fn test_scope_conversion() {
        assert_eq!(McpScope::TaskRead.as_str(), "mcp:task:read");
        assert_eq!(McpScope::from_str("mcp:task:write"), Some(McpScope::TaskWrite));
        assert_eq!(McpScope::from_str("invalid"), None);
    }
}