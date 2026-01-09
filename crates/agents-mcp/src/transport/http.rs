//! HTTP Transport for MCP
//!
//! This module provides HTTP transport for MCP servers that expose HTTP endpoints.
//! This is a generic implementation that works with any HTTP-based MCP server.
//!
//! ## Example
//!
//! ```rust,ignore
//! use agents_mcp::HttpTransport;
//!
//! // Connect to any HTTP-based MCP server
//! let transport = HttpTransport::new("https://your-mcp-server.com/mcp")
//!     .with_header("Authorization", "Bearer token")
//!     .build()?;
//!
//! let client = McpClient::connect(transport).await?;
//! ```

use crate::protocol::McpError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// HTTP Transport for MCP communication
///
/// A generic transport implementation that sends JSON-RPC requests over HTTP POST
/// and receives responses. Works with any MCP server that exposes an HTTP endpoint.
///
/// ## Features
///
/// - Standard HTTP POST for JSON-RPC requests
/// - Configurable headers for authentication
/// - Configurable timeouts
/// - Connection state tracking
pub struct HttpTransport {
    /// Base URL of the MCP server
    url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Custom headers to include in requests
    headers: HashMap<String, String>,
    /// Connection state
    connected: AtomicBool,
    /// Response buffer for receive() calls
    response_buffer: Arc<Mutex<Vec<String>>>,
}

impl HttpTransport {
    /// Create a new HTTP transport builder
    ///
    /// # Arguments
    ///
    /// * `url` - The MCP server's HTTP endpoint URL
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let transport = HttpTransport::new("https://mcp-server.example.com/api")
    ///     .with_header("Authorization", "Bearer token")
    ///     .build()?;
    /// ```
    pub fn new(url: impl Into<String>) -> HttpTransportBuilder {
        HttpTransportBuilder::new(url)
    }

    /// Get the server URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// Builder for HttpTransport
///
/// Provides a fluent API for configuring HTTP transport options.
pub struct HttpTransportBuilder {
    url: String,
    headers: HashMap<String, String>,
    timeout_secs: u64,
}

impl HttpTransportBuilder {
    /// Create a new builder with the given URL
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
            timeout_secs: 30,
        }
    }

    /// Add a custom header to all requests
    ///
    /// # Arguments
    ///
    /// * `key` - Header name
    /// * `value` - Header value
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers at once
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Set the request timeout in seconds (default: 30)
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Build the transport
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn build(self) -> Result<HttpTransport, McpError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| McpError::Transport(format!("Failed to create HTTP client: {}", e)))?;

        Ok(HttpTransport {
            url: self.url,
            client,
            headers: self.headers,
            connected: AtomicBool::new(true),
            response_buffer: Arc::new(Mutex::new(Vec::new())),
        })
    }
}

#[async_trait]
impl super::Transport for HttpTransport {
    async fn send(&mut self, message: &str) -> Result<(), McpError> {
        tracing::debug!(url = %self.url, "MCP HTTP send: {}", message);

        // Build request with required MCP headers
        let mut request = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        // Add custom headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Send request
        let response = request
            .body(message.to_string())
            .send()
            .await
            .map_err(|e| McpError::Transport(format!("HTTP request failed: {}", e)))?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(McpError::Transport(format!(
                "HTTP error {}: {}",
                status, body
            )));
        }

        // Buffer the response for receive()
        let body = response
            .text()
            .await
            .map_err(|e| McpError::Transport(format!("Failed to read response: {}", e)))?;

        tracing::debug!(url = %self.url, "MCP HTTP response: {}", body);

        // Store response in buffer
        let mut buffer = self.response_buffer.lock().await;
        buffer.push(body);

        Ok(())
    }

    async fn receive(&mut self) -> Result<String, McpError> {
        // Check for buffered response
        let mut buffer = self.response_buffer.lock().await;
        if let Some(response) = buffer.pop() {
            return Ok(response);
        }
        drop(buffer);

        // No buffered response - this shouldn't happen in normal flow
        // as send() buffers the response
        Err(McpError::Transport(
            "No response available - call send() first".to_string(),
        ))
    }

    async fn close(&mut self) -> Result<(), McpError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for HttpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpTransport")
            .field("url", &self.url)
            .field("headers", &self.headers.keys().collect::<Vec<_>>())
            .field("connected", &self.connected.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let transport = HttpTransport::new("https://example.com/mcp")
            .build()
            .unwrap();

        assert_eq!(transport.url(), "https://example.com/mcp");
        assert!(transport.is_connected());
    }

    #[test]
    fn test_builder_with_headers() {
        let transport = HttpTransport::new("https://example.com/mcp")
            .with_header("Authorization", "Bearer test-token")
            .with_header("X-Custom", "value")
            .build()
            .unwrap();

        assert_eq!(transport.headers.len(), 2);
        assert_eq!(
            transport.headers.get("Authorization"),
            Some(&"Bearer test-token".to_string())
        );
    }

    #[test]
    fn test_builder_with_timeout() {
        // Just verify it doesn't panic
        let _transport = HttpTransport::new("https://example.com/mcp")
            .with_timeout_secs(60)
            .build()
            .unwrap();
    }

    #[test]
    fn test_debug_format() {
        let transport = HttpTransport::new("https://example.com/mcp")
            .with_header("Authorization", "secret")
            .build()
            .unwrap();

        let debug_str = format!("{:?}", transport);
        // Should show header keys but not values (for security)
        assert!(debug_str.contains("Authorization"));
        assert!(!debug_str.contains("secret"));
    }
}
