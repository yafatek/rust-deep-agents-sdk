//! MCP Client Implementation
//!
//! The MCP client handles the protocol-level communication with MCP servers,
//! including initialization, tool listing, and tool execution.

use crate::protocol::{
    messages::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId},
    types::{
        InitializeParams, InitializeResult, McpTool, McpToolResult, ToolCallParams, ToolsListResult,
    },
    McpError,
};
use crate::transport::Transport;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, instrument, trace, warn};

/// Default timeout for MCP requests
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// MCP Client Configuration
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Timeout for individual requests
    pub request_timeout: Duration,

    /// Whether to automatically list tools after initialization
    pub auto_list_tools: bool,

    /// Custom client name (for initialization)
    pub client_name: Option<String>,

    /// Custom client version (for initialization)
    pub client_version: Option<String>,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: DEFAULT_TIMEOUT,
            auto_list_tools: true,
            client_name: None,
            client_version: None,
        }
    }
}

/// MCP Client
///
/// Handles communication with an MCP server, including initialization,
/// tool discovery, and tool execution.
pub struct McpClient {
    /// The underlying transport
    transport: Arc<Mutex<Box<dyn Transport>>>,

    /// Request ID counter
    request_id: AtomicU64,

    /// Client configuration
    config: McpClientConfig,

    /// Server information (after initialization)
    server_info: Option<InitializeResult>,

    /// Cached list of available tools
    tools: Vec<McpTool>,

    /// Whether the client has been initialized
    initialized: bool,
}

impl McpClient {
    /// Connect to an MCP server and perform initialization
    ///
    /// This spawns the connection, performs the MCP handshake, and
    /// optionally lists available tools.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let transport = StdioTransport::spawn("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]).await?;
    /// let client = McpClient::connect(transport).await?;
    /// ```
    #[instrument(skip(transport), name = "mcp_connect")]
    pub async fn connect<T: Transport + 'static>(transport: T) -> Result<Self, McpError> {
        Self::connect_with_config(transport, McpClientConfig::default()).await
    }

    /// Connect with custom configuration
    #[instrument(skip(transport, config), name = "mcp_connect")]
    pub async fn connect_with_config<T: Transport + 'static>(
        transport: T,
        config: McpClientConfig,
    ) -> Result<Self, McpError> {
        let mut client = Self {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config,
            server_info: None,
            tools: Vec::new(),
            initialized: false,
        };

        // Perform MCP initialization handshake
        client.initialize().await?;

        // Optionally list tools
        if client.config.auto_list_tools {
            if let Some(ref caps) = client.server_info {
                if caps.capabilities.tools.is_some() {
                    client.tools = client.list_tools_internal().await?;
                    info!(tool_count = client.tools.len(), "Discovered MCP tools");
                }
            }
        }

        Ok(client)
    }

    /// Perform the MCP initialization handshake
    async fn initialize(&mut self) -> Result<(), McpError> {
        debug!("Starting MCP initialization handshake");

        // Build initialize params
        let mut params = InitializeParams::default();
        if let Some(ref name) = self.config.client_name {
            params.client_info.name = name.clone();
        }
        if let Some(ref version) = self.config.client_version {
            params.client_info.version = version.clone();
        }

        // Send initialize request
        let result: InitializeResult = self.send_request("initialize", Some(params)).await?;

        info!(
            server_name = %result.server_info.name,
            server_version = ?result.server_info.version,
            protocol_version = %result.protocol_version,
            "MCP server initialized"
        );

        // Store server info
        self.server_info = Some(result);

        // Send initialized notification
        self.send_notification("notifications/initialized", None::<()>)
            .await?;

        self.initialized = true;
        debug!("MCP initialization complete");

        Ok(())
    }

    /// List available tools from the server
    async fn list_tools_internal(&self) -> Result<Vec<McpTool>, McpError> {
        debug!("Listing MCP tools");

        let result: ToolsListResult = self.send_request("tools/list", None::<()>).await?;

        for tool in &result.tools {
            trace!(
                tool_name = %tool.name,
                description = ?tool.description,
                "Found MCP tool"
            );
        }

        Ok(result.tools)
    }

    /// Refresh the list of available tools
    pub async fn refresh_tools(&mut self) -> Result<&[McpTool], McpError> {
        self.ensure_initialized()?;
        self.tools = self.list_tools_internal().await?;
        Ok(&self.tools)
    }

    /// Get the cached list of available tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&McpTool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Check if a tool exists
    pub fn has_tool(&self, name: &str) -> bool {
        self.get_tool(name).is_some()
    }

    /// Call a tool on the MCP server
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the tool to call
    /// * `arguments` - Arguments to pass to the tool (as JSON Value)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.call_tool("read_file", serde_json::json!({
    ///     "path": "/tmp/test.txt"
    /// })).await?;
    /// ```
    #[instrument(skip(self, arguments), fields(tool_name = %name))]
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult, McpError> {
        self.ensure_initialized()?;

        // Verify tool exists (optional, server will also validate)
        if !self.has_tool(name) {
            warn!(tool_name = %name, "Calling unknown tool");
        }

        debug!(tool_name = %name, "Calling MCP tool");

        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };

        let result: McpToolResult = self.send_request("tools/call", Some(params)).await?;

        if result.is_error {
            warn!(tool_name = %name, "Tool returned error result");
        } else {
            debug!(
                tool_name = %name,
                content_count = result.content.len(),
                "Tool call successful"
            );
        }

        Ok(result)
    }

    /// Call a tool with typed arguments
    pub async fn call_tool_typed<A: Serialize>(
        &self,
        name: &str,
        arguments: A,
    ) -> Result<McpToolResult, McpError> {
        let args = serde_json::to_value(arguments)?;
        self.call_tool(name, args).await
    }

    /// Get server information (after initialization)
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    /// Check if the client is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Check if the transport is still connected
    pub async fn is_connected(&self) -> bool {
        let transport = self.transport.lock().await;
        transport.is_connected()
    }

    /// Close the client connection
    pub async fn close(&mut self) -> Result<(), McpError> {
        debug!("Closing MCP client connection");
        let mut transport = self.transport.lock().await;
        transport.close().await?;
        self.initialized = false;
        Ok(())
    }

    // ========================================
    // Internal Helper Methods
    // ========================================

    fn ensure_initialized(&self) -> Result<(), McpError> {
        if !self.initialized {
            return Err(McpError::NotInitialized);
        }
        Ok(())
    }

    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.request_id.fetch_add(1, Ordering::SeqCst))
    }

    async fn send_request<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<R, McpError> {
        let id = self.next_request_id();

        let mut request = JsonRpcRequest::new(id.clone(), method);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        let request_json = serde_json::to_string(&request)?;
        trace!(method = %method, id = %id, "Sending JSON-RPC request");

        // CRITICAL: Hold lock for entire request/response cycle to prevent
        // concurrent requests from interleaving and causing ResponseIdMismatch.
        // This ensures atomic request-response pairs.
        let response = timeout(self.config.request_timeout, async {
            let mut transport = self.transport.lock().await;

            // Send request while holding the lock
            transport.send(&request_json).await?;

            // Loop to receive response, skipping any server notifications
            // MCP servers can emit notifications (e.g., tool list changes) at any time
            loop {
                let response_json = transport.receive().await?;

                // Try to parse as an incoming message (response or notification)
                let message: IncomingMessage = serde_json::from_str(&response_json)?;

                match message {
                    IncomingMessage::Response(response) => {
                        // Got a response - return it
                        return Ok::<JsonRpcResponse, McpError>(response);
                    }
                    IncomingMessage::Notification(notif) => {
                        // Server notification - log and continue waiting for response
                        trace!(
                            method = %notif.method,
                            "Received server notification while awaiting response, skipping"
                        );
                        continue;
                    }
                }
            }
        })
        .await
        .map_err(|_| McpError::Timeout(self.config.request_timeout))??;

        // Verify response ID matches
        if response.id != id {
            return Err(McpError::ResponseIdMismatch {
                expected: id.to_string(),
                actual: response.id.to_string(),
            });
        }

        // Extract result or error
        let result = response.into_result()?;

        // Deserialize result
        serde_json::from_value(result).map_err(McpError::from)
    }

    async fn send_notification<P: Serialize>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<(), McpError> {
        let mut notification = JsonRpcNotification::new(method);
        if let Some(p) = params {
            notification = notification.with_params(p);
        }

        let notification_json = serde_json::to_string(&notification)?;
        trace!(method = %method, "Sending JSON-RPC notification");

        let mut transport = self.transport.lock().await;
        transport.send(&notification_json).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex as StdMutex;

    #[test]
    fn test_client_config_default() {
        let config = McpClientConfig::default();
        assert_eq!(config.request_timeout, DEFAULT_TIMEOUT);
        assert!(config.auto_list_tools);
    }

    #[test]
    fn test_request_id_generation() {
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(MockTransport::new(vec![])))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: false,
        };

        let id1 = client.next_request_id();
        let id2 = client.next_request_id();

        assert_eq!(id1, RequestId::Number(1));
        assert_eq!(id2, RequestId::Number(2));
    }

    // Configurable mock transport for testing different scenarios
    struct MockTransport {
        responses: StdMutex<VecDeque<String>>,
        sent: StdMutex<Vec<String>>,
    }

    impl MockTransport {
        fn new(responses: Vec<&str>) -> Self {
            Self {
                responses: StdMutex::new(responses.into_iter().map(String::from).collect()),
                sent: StdMutex::new(Vec::new()),
            }
        }

        #[allow(dead_code)]
        fn get_sent(&self) -> Vec<String> {
            self.sent.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl Transport for MockTransport {
        async fn send(&mut self, message: &str) -> Result<(), McpError> {
            self.sent.lock().unwrap().push(message.to_string());
            Ok(())
        }

        async fn receive(&mut self) -> Result<String, McpError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| McpError::Transport("No more mock responses".into()))
        }

        async fn close(&mut self) -> Result<(), McpError> {
            Ok(())
        }

        fn is_connected(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_send_request_success() {
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        let result: serde_json::Value = client
            .send_request("tools/list", None::<()>)
            .await
            .unwrap();

        assert_eq!(result, serde_json::json!({"tools": []}));
    }

    #[tokio::test]
    async fn test_send_request_skips_notifications() {
        // Server sends a notification before the actual response
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/resources/list_changed"}"#,
            r#"{"jsonrpc":"2.0","id":1,"result":{"success":true}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        // Should skip the notifications and return the actual response
        let result: serde_json::Value = client
            .send_request("test/method", None::<()>)
            .await
            .unwrap();

        assert_eq!(result, serde_json::json!({"success": true}));
    }

    #[tokio::test]
    async fn test_send_request_id_mismatch_error() {
        // Server returns response with wrong ID
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","id":999,"result":{}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        let result: Result<serde_json::Value, _> = client
            .send_request("test/method", None::<()>)
            .await;

        assert!(matches!(result, Err(McpError::ResponseIdMismatch { .. })));
    }

    #[tokio::test]
    async fn test_send_request_json_rpc_error() {
        // Server returns a JSON-RPC error
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        let result: Result<serde_json::Value, _> = client
            .send_request("test/method", None::<()>)
            .await;

        // Should return a Protocol error (JSON-RPC errors become Protocol errors)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_tool_success() {
        // Simulate a tool call response
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Hello World"}],"isError":false}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        let args = serde_json::json!({"message": "hello"});
        let result = client.call_tool("test_tool", args).await.unwrap();

        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].as_text(), Some("Hello World"));
    }

    #[tokio::test]
    async fn test_call_tool_error_response() {
        // Simulate a tool that returns an error result
        let transport = MockTransport::new(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Tool failed: file not found"}],"isError":true}}"#,
        ]);
        let client = McpClient {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            config: McpClientConfig::default(),
            server_info: None,
            tools: Vec::new(),
            initialized: true,
        };

        let args = serde_json::json!({"path": "/nonexistent"});
        let result = client.call_tool("read_file", args).await.unwrap();

        assert!(result.is_error);
        assert!(result.content[0].as_text().unwrap().contains("file not found"));
    }
}
