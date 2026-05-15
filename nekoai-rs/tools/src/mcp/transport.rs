use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::Mutex,
};

/// A tool definition received from an MCP server.
#[derive(Debug, Clone)]
pub struct McpToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP protocol transport layer.
#[async_trait]
pub trait McpTransport: Send + Sync {
    /// Perform the initialization handshake.
    async fn initialize(&self) -> Result<(), String>;
    /// List available tools from the MCP server.
    async fn list_tools(&self) -> Result<Vec<McpToolDef>, String>;
    /// Call a tool with the given arguments.
    async fn call_tool(&self, name: &str, args: Value) -> Result<Value, String>;
}

/// JSON-RPC 2.0 request/response helpers.
fn jsonrpc_request(method: &str, params: Value, id: u64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

fn jsonrpc_id(response: &Value) -> Option<u64> {
    response.get("id").and_then(Value::as_u64)
}

fn jsonrpc_result(response: &Value) -> Result<Value, String> {
    if let Some(error) = response.get("error") {
        let msg = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(msg.to_string());
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| "missing 'result' in response".to_string())
}

/// Stdio transport: communicates with MCP server via child process stdin/stdout.
pub struct StdioTransport {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    request_id: Arc<Mutex<u64>>,
}

impl StdioTransport {
    /// Spawn a child process and create a stdio transport.
    pub async fn spawn(command: &str, args: &[String]) -> Result<Self, String> {
        let mut child: Child = Command::new(command)
            .args(args.iter().map(|s| s.as_str()))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("failed to spawn MCP server '{}': {}", command, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to capture stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "failed to capture stdout".to_string())?;

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            request_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Send a JSON-RPC request and read the response.
    async fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let mut id_lock = self.request_id.lock().await;
        *id_lock += 1;
        let id = *id_lock;

        let request = jsonrpc_request(method, params, id);
        let request_str =
            serde_json::to_string(&request).map_err(|e| format!("serialization error: {}", e))?;

        // Write to stdin
        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(format!("{}\n", request_str).as_bytes())
                .await
                .map_err(|e| format!("write error: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("flush error: {}", e))?;
        }

        // Read response line from stdout
        let mut line = String::new();
        {
            let mut stdout = self.stdout.lock().await;
            stdout
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read error: {}", e))?;
        }

        if line.trim().is_empty() {
            return Err("empty response from MCP server".to_string());
        }

        let response: Value =
            serde_json::from_str(line.trim()).map_err(|e| format!("parse error: {}", e))?;

        // Verify the response ID matches
        if jsonrpc_id(&response) != Some(id) {
            return Err(format!(
                "response ID mismatch: expected {}, got {:?}",
                id,
                jsonrpc_id(&response)
            ));
        }

        jsonrpc_result(&response)
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn initialize(&self) -> Result<(), String> {
        let params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "nekoai",
                "version": "1.0.0"
            }
        });
        let result = self.send_request("initialize", params).await?;
        let _version = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDef>, String> {
        let result = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;

        let tools_arr = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing 'tools' array in response".to_string())?;

        let tools = tools_arr
            .iter()
            .map(|t| McpToolDef {
                name: t
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                description: t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input_schema: t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or(serde_json::json!({})),
            })
            .collect();

        Ok(tools)
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        let params = serde_json::json!({
            "name": name,
            "arguments": args
        });
        self.send_request("tools/call", params).await
    }
}

/// SSE transport: communicates with MCP server via HTTP SSE endpoint.
pub struct SseTransport {
    url: String,
    client: reqwest::Client,
}

impl SseTransport {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl McpTransport for SseTransport {
    async fn initialize(&self) -> Result<(), String> {
        let params = serde_json::json!({
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "nekoai",
                "version": "1.0.0"
            }
        });
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": params
        });
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("SSE initialize request failed: {}", e))?;
        let _response: Value = resp
            .json()
            .await
            .map_err(|e| format!("SSE initialize parse failed: {}", e))?;
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDef>, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("SSE list tools request failed: {}", e))?;
        let result: Value = resp
            .json()
            .await
            .map_err(|e| format!("SSE list tools parse failed: {}", e))?;

        let result = jsonrpc_result(&result)?;
        let tools_arr = result
            .get("tools")
            .and_then(Value::as_array)
            .ok_or_else(|| "missing 'tools' array".to_string())?;

        let tools = tools_arr
            .iter()
            .map(|t| McpToolDef {
                name: t
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                description: t
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                input_schema: t
                    .get("inputSchema")
                    .cloned()
                    .unwrap_or(serde_json::json!({})),
            })
            .collect();
        Ok(tools)
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": args
            }
        });
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("SSE call tool request failed: {}", e))?;
        let result: Value = resp
            .json()
            .await
            .map_err(|e| format!("SSE call tool parse failed: {}", e))?;
        jsonrpc_result(&result)
    }
}
