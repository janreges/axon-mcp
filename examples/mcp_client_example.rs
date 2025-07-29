//! Simple MCP Client Example
//! 
//! Demonstrates how to connect to an MCP server and perform basic operations
//! using the HTTP POST + SSE transport pattern.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use url::Url;

#[derive(Serialize, Debug)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    id: u64,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
    id: u64,
}

struct SimpleMcpClient {
    http_client: Client,
    base_url: Url,
    request_counter: u64,
}

impl SimpleMcpClient {
    fn new(base_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(SimpleMcpClient {
            http_client: Client::new(),
            base_url: Url::parse(base_url)?,
            request_counter: 0,
        })
    }

    async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<u64, Box<dyn std::error::Error>> {
        self.request_counter += 1;
        let request_id = self.request_counter;
        
        let request_url = self.base_url.join("/mcp/request")?;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: request_id,
        };

        println!("🚀 Sending: {} (ID: {})", method, request_id);
        println!("   Payload: {}", serde_json::to_string_pretty(&request)?);

        let response = self.http_client
            .post(request_url)
            .header("Content-Type", "application/json")
            .header("Origin", self.base_url.origin().ascii_serialization())
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Request sent successfully");
        } else {
            println!("❌ Request failed: {}", response.status());
        }

        Ok(request_id)
    }

    async fn listen_sse_responses(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sse_url = self.base_url.join("/mcp/v1")?;
        
        println!("🔄 Connecting to SSE stream: {}", sse_url);
        
        let response = self.http_client
            .get(sse_url)
            .header("Accept", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Origin", self.base_url.origin().ascii_serialization())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("SSE connection failed: {}", response.status()).into());
        }

        println!("✅ SSE connection established");
        
        let stream = response.bytes_stream();
        let reader = BufReader::new(stream.map(|r| r.map_err(std::io::Error::other)).into_async_read());
        let mut lines = reader.lines();
        
        let mut event_data = String::new();
        
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            
            if line.starts_with("data: ") {
                event_data = line[6..].to_string();
            } else if line.is_empty() && !event_data.is_empty() {
                // End of event
                if event_data == "heartbeat" || event_data.is_empty() {
                    println!("💓 Heartbeat received");
                } else {
                    // Try to parse as JSON-RPC response
                    match serde_json::from_str::<JsonRpcResponse>(&event_data) {
                        Ok(response) => {
                            println!("📨 Response received (ID: {})", response.id);
                            if let Some(result) = response.result {
                                println!("   Result: {}", serde_json::to_string_pretty(&result)?);
                            } else if let Some(error) = response.error {
                                println!("   Error: {}", serde_json::to_string_pretty(&error)?);
                            }
                        }
                        Err(_) => {
                            println!("📨 Raw event data: {}", event_data);
                        }
                    }
                }
                event_data.clear();
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 MCP Client Example");
    println!("====================");
    
    // Initialize client
    let server_url = "http://127.0.0.1:8080";
    let mut client = SimpleMcpClient::new(server_url)?;
    
    // Start SSE listener in background task
    let sse_client = SimpleMcpClient::new(server_url)?;
    let sse_handle = tokio::spawn(async move {
        if let Err(e) = sse_client.listen_sse_responses().await {
            eprintln!("SSE error: {}", e);
        }
    });
    
    // Wait for SSE connection to be established
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    println!("\n🔧 Testing MCP Operations");
    println!("========================");
    
    // 1. Health Check
    println!("\n1️⃣ Health Check");
    client.send_request("health_check", None).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 2. Create a task
    println!("\n2️⃣ Create Task");
    let create_params = json!({
        "code": "EXAMPLE-001",
        "name": "Example Task",
        "description": "This is a sample task created by the example client",
        "owner_agent_name": "example-client"
    });
    client.send_request("create_task", Some(create_params)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 3. List tasks
    println!("\n3️⃣ List Tasks");
    let list_params = json!({
        "owner_agent_name": "example-client"
    });
    client.send_request("list_tasks", Some(list_params)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 4. Update task (assuming ID 1 exists)
    println!("\n4️⃣ Update Task");
    let update_params = json!({
        "id": 1,
        "name": "Updated Example Task",
        "description": "This task has been updated by the example client"
    });
    client.send_request("update_task", Some(update_params)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 5. Change task state
    println!("\n5️⃣ Set Task State");
    let state_params = json!({
        "id": 1,
        "state": "InProgress"
    });
    client.send_request("set_task_state", Some(state_params)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 6. Get task by ID
    println!("\n6️⃣ Get Task by ID");
    let get_params = json!({
        "id": 1
    });
    client.send_request("get_task_by_id", Some(get_params)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 7. Test error handling
    println!("\n7️⃣ Test Error Handling");
    client.send_request("invalid_method", None).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // 8. Final task list
    println!("\n8️⃣ Final Task List");
    client.send_request("list_tasks", Some(json!({}))).await?;
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    println!("\n✅ Example completed! Check the responses above.");
    println!("💡 The SSE connection will remain open to show any additional events.");
    println!("   Press Ctrl+C to exit.");
    
    // Keep the program running to continue receiving SSE events
    sse_handle.await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = SimpleMcpClient::new("http://localhost:8080");
        assert!(client.is_ok());
    }

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "test_method".to_string(),
            params: Some(json!({"key": "value"})),
            id: 1,
        };
        
        let serialized = serde_json::to_string(&request).unwrap();
        assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(serialized.contains("\"method\":\"test_method\""));
        assert!(serialized.contains("\"id\":1"));
    }
}