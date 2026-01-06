//! Stdio Transport for MCP
//!
//! This transport spawns an MCP server as a subprocess and communicates
//! with it via stdin/stdout using newline-delimited JSON.

use crate::protocol::McpError;
use crate::transport::Transport;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, error, trace, warn};

/// Stdio Transport Configuration
#[derive(Debug, Clone)]
pub struct StdioConfig {
    /// Command to run (e.g., "npx", "python", "node")
    pub command: String,

    /// Arguments for the command
    pub args: Vec<String>,

    /// Environment variables to set
    pub env: HashMap<String, String>,

    /// Working directory for the process
    pub working_dir: Option<String>,
}

impl StdioConfig {
    /// Create a new stdio configuration
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            working_dir: None,
        }
    }

    /// Add an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Set an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }
}

/// Stdio Transport
///
/// Spawns an MCP server as a child process and communicates via stdin/stdout.
pub struct StdioTransport {
    /// Child process handle
    child: Child,

    /// Stdin writer
    stdin: ChildStdin,

    /// Stdout reader (buffered for line reading)
    stdout: BufReader<ChildStdout>,

    /// Whether the transport is connected
    connected: bool,

    /// Server command (for debug/error messages)
    command_str: String,
}

impl StdioTransport {
    /// Spawn a new MCP server process
    ///
    /// # Arguments
    ///
    /// * `command` - The command to run (e.g., "npx", "python")
    /// * `args` - Arguments to pass to the command
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let transport = StdioTransport::spawn("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]).await?;
    /// ```
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self, McpError> {
        let config = StdioConfig::new(command).args(args.iter().copied());
        Self::spawn_with_config(config).await
    }

    /// Spawn a new MCP server process with full configuration
    pub async fn spawn_with_config(config: StdioConfig) -> Result<Self, McpError> {
        let command_str = format!("{} {}", config.command, config.args.join(" "));
        debug!(command = %command_str, "Spawning MCP server process");

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Let stderr pass through for debugging
            .kill_on_drop(true); // Ensure process is killed when transport is dropped

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref dir) = config.working_dir {
            cmd.current_dir(dir);
        }

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            error!(error = %e, command = %command_str, "Failed to spawn MCP server");
            McpError::ProcessSpawn(format!("{}: {}", command_str, e))
        })?;

        // Get stdin and stdout handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::ProcessSpawn("Failed to capture stdin".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::ProcessSpawn("Failed to capture stdout".to_string()))?;

        debug!(command = %command_str, "MCP server process spawned successfully");

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            connected: true,
            command_str,
        })
    }

    /// Check if the child process is still running
    pub fn check_process(&mut self) -> Result<(), McpError> {
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.connected = false;
                warn!(
                    command = %self.command_str,
                    exit_code = ?status.code(),
                    "MCP server process exited"
                );
                Err(McpError::ProcessExited)
            }
            Ok(None) => Ok(()), // Still running
            Err(e) => {
                self.connected = false;
                Err(McpError::Io(e))
            }
        }
    }

    /// Kill the child process
    pub async fn kill(&mut self) -> Result<(), McpError> {
        debug!(command = %self.command_str, "Killing MCP server process");
        self.child.kill().await.map_err(McpError::Io)?;
        self.connected = false;
        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, message: &str) -> Result<(), McpError> {
        // Check if process is still running
        self.check_process()?;

        trace!(message = %message, "Sending message to MCP server");

        // Write message followed by newline
        self.stdin
            .write_all(message.as_bytes())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to write to MCP server stdin");
                McpError::Transport(format!("Write failed: {}", e))
            })?;

        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| McpError::Transport(format!("Write newline failed: {}", e)))?;

        self.stdin
            .flush()
            .await
            .map_err(|e| McpError::Transport(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<String, McpError> {
        // Check if process is still running
        self.check_process()?;

        let mut line = String::new();

        // Read a line from stdout
        let bytes_read = self.stdout.read_line(&mut line).await.map_err(|e| {
            error!(error = %e, "Failed to read from MCP server stdout");
            McpError::Transport(format!("Read failed: {}", e))
        })?;

        if bytes_read == 0 {
            self.connected = false;
            return Err(McpError::ProcessExited);
        }

        // Remove trailing newline
        let line = line.trim_end().to_string();
        trace!(message = %line, "Received message from MCP server");

        Ok(line)
    }

    async fn close(&mut self) -> Result<(), McpError> {
        if self.connected {
            debug!(command = %self.command_str, "Closing MCP server connection");

            // Give the process a moment to exit gracefully after we stop writing
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Kill if still running
            if self.check_process().is_ok() {
                self.kill().await?;
            }
        }
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if self.connected {
            // Try to kill the process on drop
            // Note: This is best-effort since we can't await in drop
            let _ = self.child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_config() {
        let config = StdioConfig::new("npx")
            .args(["-y", "@modelcontextprotocol/server-filesystem"])
            .arg("/tmp")
            .env("DEBUG", "true")
            .working_dir("/home/user");

        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 3);
        assert_eq!(config.env.get("DEBUG"), Some(&"true".to_string()));
        assert_eq!(config.working_dir, Some("/home/user".to_string()));
    }
}
