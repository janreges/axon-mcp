//! Structured request logging middleware for MCP HTTP server
//!
//! Provides single-line formatted request logging with timing and parameter truncation.

use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use http_body_util::BodyExt;
use serde_json::Value;
use std::time::Instant;

/// Maximum length for parameter values before truncation
const MAX_PARAM_LENGTH: usize = 30;

/// Truncation suffix for long parameters
const TRUNCATION_SUFFIX: &str = "...";

/// MCP request logging middleware
///
/// Logs requests in the format:
/// `YYYY-MM-DD HH:MM:SS [functionName] [34 ms] param1="value", param2="long value..."`
pub async fn mcp_request_logging_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    
    // Only log MCP endpoints
    if !path.starts_with("/mcp") {
        return Ok(next.run(request).await);
    }

    // Extract request body for JSON-RPC parsing
    let (parts, body) = request.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            // If we can't read the body, proceed without logging details
            let new_request = Request::from_parts(parts, Body::empty());
            let response = next.run(new_request).await;
            let duration = start_time.elapsed();
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
            println!(
                "{} [UNKNOWN] [{} ms] {} {} - Body read error",
                timestamp,
                duration.as_millis(),
                method,
                path
            );
            return Ok(response);
        }
    };

    // Parse JSON-RPC request
    let (function_name, params_summary) = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(json) => {
            let method_name = json.get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown");
            
            let params = json.get("params").unwrap_or(&Value::Null);
            let params_str = format_params_summary(params);
            
            (method_name.to_string(), params_str)
        }
        Err(_) => ("invalid_json".to_string(), String::new())
    };

    // Reconstruct request with body
    let new_body = Body::from(body_bytes);
    let new_request = Request::from_parts(parts, new_body);

    // Process request
    let response = next.run(new_request).await;
    let duration = start_time.elapsed();
    let _status = response.status();
    
    // Format timestamp
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
    
    // Log with MCP function details
    if params_summary.is_empty() {
        println!(
            "{} [{}] [{} ms]",
            timestamp,
            function_name,
            duration.as_millis()
        );
    } else {
        println!(
            "{} [{}] [{} ms] {}",
            timestamp,
            function_name,
            duration.as_millis(),
            params_summary
        );
    }
    
    Ok(response)
}

/// Format parameters into a summary string with truncation
fn format_params_summary(params: &Value) -> String {
    match params {
        Value::Object(map) => {
            let formatted_params: Vec<String> = map
                .iter()  
                .filter_map(|(key, value)| {
                    // Skip sensitive parameters
                    if is_sensitive_parameter(key) {
                        return Some(format!("{}=\"[REDACTED]\"", key));
                    }
                    
                    let formatted_value = format_parameter_value(value);
                    if formatted_value.is_empty() {
                        None
                    } else {
                        Some(format!("{}=\"{}\"", key, formatted_value))
                    }
                })
                .collect();
            
            formatted_params.join(", ")
        }
        Value::Array(arr) => {
            format!("[{} items]", arr.len())
        }
        _ => format_parameter_value(params),
    }
}

/// Format a single parameter value with truncation
fn format_parameter_value(value: &Value) -> String {
    let value_str = match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(obj) => format!("{{...{} fields}}", obj.len()),
        Value::Null => "null".to_string(),
    };
    
    truncate_string(&value_str, MAX_PARAM_LENGTH)
}

/// Truncate string to max length with suffix
fn truncate_string(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        input.to_string()
    } else {
        let truncated_length = max_length.saturating_sub(TRUNCATION_SUFFIX.len());
        format!("{}{}", &input[..truncated_length], TRUNCATION_SUFFIX)
    }
}

/// Check if parameter should be redacted for security
fn is_sensitive_parameter(key: &str) -> bool {
    let sensitive_keys = [
        "password", "token", "secret", "key", "auth", "authorization",
        "credential", "api_key", "access_token", "refresh_token"
    ];
    
    let key_lower = key.to_lowercase();
    sensitive_keys.iter().any(|&sensitive| key_lower.contains(sensitive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
        assert_eq!(truncate_string("exactly10c", 10), "exactly10c");
    }

    #[test]
    fn test_format_parameter_value() {
        assert_eq!(format_parameter_value(&json!("test")), "test");
        assert_eq!(format_parameter_value(&json!(42)), "42");
        assert_eq!(format_parameter_value(&json!(true)), "true");
        assert_eq!(format_parameter_value(&json!(null)), "null");
        assert_eq!(format_parameter_value(&json!(["a", "b", "c"])), "[3 items]");
    }

    #[test]
    fn test_format_params_summary() {
        let params = json!({
            "code": "TASK-001",
            "name": "This is a very long task name that should be truncated",
            "count": 42,
            "enabled": true
        });
        
        let summary = format_params_summary(&params);
        assert!(summary.contains("code=\"TASK-001\""));
        assert!(summary.contains("name=\"This is a very long task na...\""));
        assert!(summary.contains("count=\"42\""));
        assert!(summary.contains("enabled=\"true\""));
    }

    #[test]
    fn test_sensitive_parameter_redaction() {
        let params = json!({
            "username": "user123",
            "password": "secret123",
            "api_key": "sensitive_key",
            "normal_field": "normal_value"
        });
        
        let summary = format_params_summary(&params);
        assert!(summary.contains("username=\"user123\""));
        assert!(summary.contains("password=\"[REDACTED]\""));
        assert!(summary.contains("api_key=\"[REDACTED]\""));
        assert!(summary.contains("normal_field=\"normal_value\""));
    }

    #[test]
    fn test_is_sensitive_parameter() {
        assert!(is_sensitive_parameter("password"));
        assert!(is_sensitive_parameter("api_key"));
        assert!(is_sensitive_parameter("access_token"));
        assert!(is_sensitive_parameter("AUTH_TOKEN"));
        assert!(!is_sensitive_parameter("username"));
        assert!(!is_sensitive_parameter("name"));
        assert!(!is_sensitive_parameter("description"));
    }
}