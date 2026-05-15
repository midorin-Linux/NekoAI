use std::sync::Arc;

use nekoai_config::loader::McpServerConfig;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tracing::info;

use super::transport::{McpToolDef, McpTransport, SseTransport, StdioTransport};

/// An MCP client that connects to an MCP server and exposes its tools.
pub struct McpClient {
    name: String,
    transport: Arc<dyn McpTransport>,
    tools: Arc<RwLock<Vec<McpToolDef>>>,
    connected: Arc<RwLock<bool>>,
}

impl McpClient {
    /// Connect to an MCP server based on its configuration.
    pub async fn connect(config: &McpServerConfig) -> Result<Self, String> {
        let transport: Arc<dyn McpTransport> = match config.transport.as_str() {
            "stdio" => {
                let command = config
                    .command
                    .as_ref()
                    .ok_or_else(|| "stdio transport requires 'command'".to_string())?;
                let args: Vec<String> = config.args.clone().unwrap_or_default();
                Arc::new(
                    StdioTransport::spawn(command, &args)
                        .await
                        .map_err(|e| format!("MCP '{}' stdio spawn failed: {}", config.name, e))?,
                )
            }
            "sse" => {
                let url = config
                    .url
                    .as_ref()
                    .ok_or_else(|| "sse transport requires 'url'".to_string())?;
                Arc::new(SseTransport::new(url.clone()))
            }
            other => return Err(format!("unsupported MCP transport: {}", other)),
        };

        let client = Self {
            name: config.name.clone(),
            transport,
            tools: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(RwLock::new(false)),
        };

        client.initialize().await?;
        Ok(client)
    }

    /// Initialize the MCP connection and fetch available tools.
    pub async fn initialize(&self) -> Result<(), String> {
        self.transport
            .initialize()
            .await
            .map_err(|e| format!("MCP '{}' init failed: {}", self.name, e))?;

        let tools = self
            .transport
            .list_tools()
            .await
            .map_err(|e| format!("MCP '{}' list_tools failed: {}", self.name, e))?;

        info!(
            mcp_server = self.name,
            tool_count = tools.len(),
            "MCP server connected"
        );

        *self.tools.write().await = tools;
        *self.connected.write().await = true;
        Ok(())
    }

    /// Get the list of tool definitions.
    pub async fn tool_defs(&self) -> Vec<McpToolDef> {
        self.tools.read().await.clone()
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Call a tool by name with the given arguments.
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        self.transport.call_tool(name, args).await
    }
}

/// A rig Tool wrapper for an MCP tool.
pub struct McpToolWrapper {
    server_name: String,
    client: Arc<McpClient>,
    def: McpToolDef,
}

impl McpToolWrapper {
    pub fn new(client: Arc<McpClient>, def: McpToolDef) -> Self {
        let server_name = client.name().to_string();
        Self {
            server_name,
            client,
            def,
        }
    }
}

impl Tool for McpToolWrapper {
    const NAME: &'static str = "mcp_tool"; // Overridden by name()

    type Error = StringError;
    type Args = Value;
    type Output = Value;

    fn name(&self) -> String {
        format!("mcp_{}_{}", self.server_name, self.def.name)
    }

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.name(),
            description: format!("[MCP: {}] {}", self.server_name, self.def.description),
            parameters: self.def.input_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        tracing::info!(
            target: "nekoai-tools",
            mcp_server = self.server_name,
            tool = self.def.name,
            "MCP tool called"
        );

        let args_to_pass = args.get("arguments").cloned().unwrap_or(args);

        match self.client.call_tool(&self.def.name, args_to_pass).await {
            Ok(result) => Ok(json!({
                "ok": true,
                "data": result
            })),
            Err(e) => Ok(json!({
                "ok": false,
                "error": format!("MCP tool '{}' failed: {}", self.def.name, e)
            })),
        }
    }
}

#[derive(Debug)]
pub struct StringError(pub String);

impl std::fmt::Display for StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StringError {}
