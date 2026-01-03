//! TOON (Token-Oriented Object Notation) encoding support
//!
//! This module provides utilities for encoding data in TOON format,
//! a compact, human-readable format that reduces token usage when
//! sending structured data to LLMs.
//!
//! TOON typically provides 30-60% token reduction compared to JSON,
//! making it ideal for LLM prompts and tool descriptions.
//!
//! See: <https://github.com/toon-format/toon>
//!
//! # Example
//!
//! ```rust,ignore
//! use agents_core::toon::ToonEncoder;
//! use serde_json::json;
//!
//! let encoder = ToonEncoder::default();
//! let data = json!({
//!     "users": [
//!         {"id": 1, "name": "Alice"},
//!         {"id": 2, "name": "Bob"}
//!     ]
//! });
//!
//! let toon_str = encoder.encode(&data).unwrap();
//! // Output:
//! // users[2]{id,name}:
//! //   1,Alice
//! //   2,Bob
//! ```

use serde::Serialize;

#[cfg(feature = "toon")]
use toon_format::{encode_default, EncodeOptions, ToonError};

/// TOON encoder for converting data to token-efficient format
///
/// When the `toon` feature is enabled, this encoder uses the official
/// TOON format implementation. When disabled, it falls back to JSON.
#[derive(Debug, Clone, Default)]
pub struct ToonEncoder {
    /// Use tab delimiter for even more compact output
    pub use_tabs: bool,
    /// Enable key folding for nested objects (e.g., `data.user.name: Alice`)
    pub fold_keys: bool,
}

impl ToonEncoder {
    /// Create a new TOON encoder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an encoder optimized for maximum token savings
    pub fn compact() -> Self {
        Self {
            use_tabs: true,
            fold_keys: true,
        }
    }

    /// Set whether to use tab delimiters
    pub fn with_tabs(mut self, use_tabs: bool) -> Self {
        self.use_tabs = use_tabs;
        self
    }

    /// Set whether to fold nested keys
    pub fn with_key_folding(mut self, fold_keys: bool) -> Self {
        self.fold_keys = fold_keys;
        self
    }

    /// Encode a value to TOON format
    ///
    /// When the `toon` feature is enabled, this produces compact TOON output.
    /// When disabled, it falls back to pretty-printed JSON.
    #[cfg(feature = "toon")]
    pub fn encode<T: Serialize>(&self, value: &T) -> Result<String, ToonEncodeError> {
        let options = self.build_options();
        toon_format::encode(value, &options).map_err(ToonEncodeError::from)
    }

    /// Encode a value to TOON format (fallback to JSON when toon feature disabled)
    #[cfg(not(feature = "toon"))]
    pub fn encode<T: Serialize>(&self, value: &T) -> Result<String, ToonEncodeError> {
        serde_json::to_string_pretty(value).map_err(ToonEncodeError::from)
    }

    /// Encode a value using default options
    #[cfg(feature = "toon")]
    pub fn encode_default<T: Serialize>(value: &T) -> Result<String, ToonEncodeError> {
        encode_default(value).map_err(ToonEncodeError::from)
    }

    /// Encode a value using default options (fallback to JSON)
    #[cfg(not(feature = "toon"))]
    pub fn encode_default<T: Serialize>(value: &T) -> Result<String, ToonEncodeError> {
        serde_json::to_string_pretty(value).map_err(ToonEncodeError::from)
    }

    /// Encode a JSON value to TOON format
    pub fn encode_json(&self, value: &serde_json::Value) -> Result<String, ToonEncodeError> {
        self.encode(value)
    }

    /// Build encoding options
    #[cfg(feature = "toon")]
    fn build_options(&self) -> EncodeOptions {
        use toon_format::types::{Delimiter, KeyFoldingMode};
        let mut options = EncodeOptions::default();

        if self.use_tabs {
            options = options.with_delimiter(Delimiter::Tab);
        }

        if self.fold_keys {
            options = options.with_key_folding(KeyFoldingMode::Safe);
        }

        options
    }
}

/// Error type for TOON encoding
#[derive(Debug)]
pub enum ToonEncodeError {
    /// Error during TOON encoding
    #[cfg(feature = "toon")]
    Toon(ToonError),
    /// Error during JSON serialization (fallback mode)
    Json(serde_json::Error),
}

impl std::fmt::Display for ToonEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "toon")]
            ToonEncodeError::Toon(e) => write!(f, "TOON encoding error: {}", e),
            ToonEncodeError::Json(e) => write!(f, "JSON encoding error: {}", e),
        }
    }
}

impl std::error::Error for ToonEncodeError {}

#[cfg(feature = "toon")]
impl From<ToonError> for ToonEncodeError {
    fn from(err: ToonError) -> Self {
        ToonEncodeError::Toon(err)
    }
}

impl From<serde_json::Error> for ToonEncodeError {
    fn from(err: serde_json::Error) -> Self {
        ToonEncodeError::Json(err)
    }
}

/// Convert a tool schema to TOON format for compact prompt inclusion
///
/// This is useful when describing tools in system prompts rather than
/// using the provider's native tool API.
#[cfg(feature = "toon")]
pub fn tool_schema_to_toon(schema: &crate::tools::ToolSchema) -> Result<String, ToonEncodeError> {
    use serde_json::json;

    let value = json!({
        "name": schema.name,
        "description": schema.description,
        "parameters": schema.parameters
    });

    ToonEncoder::encode_default(&value)
}

/// Convert a tool schema to TOON format (fallback)
#[cfg(not(feature = "toon"))]
pub fn tool_schema_to_toon(schema: &crate::tools::ToolSchema) -> Result<String, ToonEncodeError> {
    serde_json::to_string_pretty(schema).map_err(ToonEncodeError::from)
}

/// Format tool call examples in TOON format for prompts
///
/// # Example
///
/// ```rust,ignore
/// use agents_core::toon::format_tool_call_toon;
///
/// let example = format_tool_call_toon("search", &json!({"query": "rust", "limit": 10}));
/// // Output:
/// // tool: search
/// // args:
/// //   query: rust
/// //   limit: 10
/// ```
#[cfg(feature = "toon")]
pub fn format_tool_call_toon(
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String, ToonEncodeError> {
    use serde_json::json;

    let value = json!({
        "tool": tool_name,
        "args": args
    });

    ToonEncoder::encode_default(&value)
}

/// Format tool call examples in TOON format (fallback to JSON)
#[cfg(not(feature = "toon"))]
pub fn format_tool_call_toon(
    tool_name: &str,
    args: &serde_json::Value,
) -> Result<String, ToonEncodeError> {
    use serde_json::json;

    let value = json!({
        "tool": tool_name,
        "args": args
    });

    serde_json::to_string_pretty(&value).map_err(ToonEncodeError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_encoder_basic() {
        let encoder = ToonEncoder::new();
        let data = json!({"name": "Alice", "age": 30});
        let result = encoder.encode(&data).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_encoder_array() {
        let encoder = ToonEncoder::new();
        let data = json!({
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });
        let result = encoder.encode(&data).unwrap();
        assert!(!result.is_empty());

        #[cfg(feature = "toon")]
        {
            // TOON format uses compact tabular notation
            assert!(result.contains("users"));
        }
    }

    #[test]
    fn test_encode_default() {
        let data = json!({"key": "value"});
        let result = ToonEncoder::encode_default(&data).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_tool_call() {
        let result = format_tool_call_toon("search", &json!({"query": "rust"})).unwrap();
        assert!(result.contains("search"));
        assert!(result.contains("rust"));
    }
}
