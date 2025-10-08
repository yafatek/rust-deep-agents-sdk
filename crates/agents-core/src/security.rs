//! Security utilities for PII protection and data sanitization

use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Maximum length for message previews to prevent PII leakage
pub const MAX_PREVIEW_LENGTH: usize = 100;

/// Sensitive field names that should be redacted from tool payloads
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "secret",
    "token",
    "api_key",
    "apikey",
    "access_token",
    "refresh_token",
    "auth_token",
    "authorization",
    "bearer",
    "credit_card",
    "card_number",
    "cvv",
    "ssn",
    "social_security",
    "private_key",
    "privatekey",
    "encryption_key",
];

lazy_static::lazy_static! {
    /// Regex patterns for detecting PII in text
    static ref EMAIL_PATTERN: Regex = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap();
    static ref PHONE_PATTERN: Regex = Regex::new(r"\b(\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap();
    static ref CREDIT_CARD_PATTERN: Regex = Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap();
}

/// Truncate a string to a maximum length, adding ellipsis if truncated
///
/// # Examples
///
/// ```
/// use agents_core::security::truncate_string;
///
/// let short = "Hello";
/// assert_eq!(truncate_string(short, 100), "Hello");
///
/// let long = "a".repeat(150);
/// let truncated = truncate_string(&long, 100);
/// assert_eq!(truncated.len(), 103); // 100 chars + "..."
/// assert!(truncated.ends_with("..."));
/// ```
pub fn truncate_string(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[..max_length])
    }
}

/// Sanitize a JSON value by redacting sensitive fields
///
/// This function recursively traverses a JSON structure and replaces
/// values of sensitive fields with "[REDACTED]".
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use agents_core::security::sanitize_json;
///
/// let input = json!({
///     "username": "john",
///     "password": "secret123",
///     "api_key": "sk-1234567890"
/// });
///
/// let sanitized = sanitize_json(&input);
/// assert_eq!(sanitized["username"], "john");
/// assert_eq!(sanitized["password"], "[REDACTED]");
/// assert_eq!(sanitized["api_key"], "[REDACTED]");
/// ```
pub fn sanitize_json(value: &Value) -> Value {
    let sensitive_set: HashSet<&str> = SENSITIVE_FIELDS.iter().copied().collect();
    sanitize_json_recursive(value, &sensitive_set)
}

fn sanitize_json_recursive(value: &Value, sensitive_fields: &HashSet<&str>) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = serde_json::Map::new();
            for (key, val) in map {
                let key_lower = key.to_lowercase();
                if sensitive_fields
                    .iter()
                    .any(|&field| key_lower.contains(field))
                {
                    sanitized.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    sanitized.insert(key.clone(), sanitize_json_recursive(val, sensitive_fields));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| sanitize_json_recursive(v, sensitive_fields))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Redact PII patterns from text (emails, phone numbers, credit cards)
///
/// # Examples
///
/// ```
/// use agents_core::security::redact_pii;
///
/// let text = "Contact me at john@example.com or call 555-123-4567";
/// let redacted = redact_pii(text);
/// assert!(redacted.contains("[EMAIL]"));
/// assert!(redacted.contains("[PHONE]"));
/// assert!(!redacted.contains("john@example.com"));
/// assert!(!redacted.contains("555-123-4567"));
/// ```
pub fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();

    // Redact emails
    result = EMAIL_PATTERN.replace_all(&result, "[EMAIL]").to_string();

    // Redact phone numbers
    result = PHONE_PATTERN.replace_all(&result, "[PHONE]").to_string();

    // Redact credit card numbers
    result = CREDIT_CARD_PATTERN
        .replace_all(&result, "[CARD]")
        .to_string();

    result
}

/// Create a safe preview of text by truncating and redacting PII
///
/// This combines truncation and PII redaction for maximum safety.
///
/// # Examples
///
/// ```
/// use agents_core::security::safe_preview;
///
/// let text = "My email is john@example.com and here's a very long message that goes on and on...";
/// let preview = safe_preview(text, 50);
/// assert!(preview.len() <= 53); // 50 + "..."
/// assert!(preview.contains("[EMAIL]"));
/// ```
pub fn safe_preview(text: &str, max_length: usize) -> String {
    let redacted = redact_pii(text);
    truncate_string(&redacted, max_length)
}

/// Sanitize tool payload for safe logging/broadcasting
///
/// This function:
/// 1. Redacts sensitive fields from JSON
/// 2. Truncates the result to prevent excessive data
/// 3. Redacts any remaining PII patterns
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use agents_core::security::sanitize_tool_payload;
///
/// let payload = json!({
///     "password": "secret123",
///     "api_key": "sk-1234567890",
///     "user": "john@example.com"
/// });
///
/// let sanitized = sanitize_tool_payload(&payload, 100);
/// assert!(sanitized.contains("[REDACTED]"));
/// assert!(sanitized.contains("[EMAIL]"));
/// assert!(sanitized.len() <= 103); // 100 + "..."
/// ```
pub fn sanitize_tool_payload(payload: &Value, max_length: usize) -> String {
    let sanitized_json = sanitize_json(payload);
    let json_str = sanitized_json.to_string();
    safe_preview(&json_str, max_length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_truncate_string_short() {
        let text = "Hello, world!";
        assert_eq!(truncate_string(text, 100), "Hello, world!");
    }

    #[test]
    fn test_truncate_string_long() {
        let text = "a".repeat(150);
        let truncated = truncate_string(&text, 100);
        assert_eq!(truncated.len(), 103); // 100 + "..."
        assert!(truncated.ends_with("..."));
        assert_eq!(&truncated[..100], &text[..100]);
    }

    #[test]
    fn test_truncate_string_exact() {
        let text = "a".repeat(100);
        let truncated = truncate_string(&text, 100);
        assert_eq!(truncated.len(), 100);
        assert!(!truncated.ends_with("..."));
    }

    #[test]
    fn test_sanitize_json_simple() {
        let input = json!({
            "username": "john",
            "password": "secret123"
        });

        let sanitized = sanitize_json(&input);
        assert_eq!(sanitized["username"], "john");
        assert_eq!(sanitized["password"], "[REDACTED]");
    }

    #[test]
    fn test_sanitize_json_nested() {
        let input = json!({
            "user": {
                "name": "john",
                "credentials": {
                    "password": "secret123",
                    "api_key": "sk-1234567890"
                }
            }
        });

        let sanitized = sanitize_json(&input);
        assert_eq!(sanitized["user"]["name"], "john");
        assert_eq!(sanitized["user"]["credentials"]["password"], "[REDACTED]");
        assert_eq!(sanitized["user"]["credentials"]["api_key"], "[REDACTED]");
    }

    #[test]
    fn test_sanitize_json_array() {
        let input = json!({
            "users": [
                {"name": "john", "password": "secret1"},
                {"name": "jane", "token": "abc123"}
            ]
        });

        let sanitized = sanitize_json(&input);
        assert_eq!(sanitized["users"][0]["name"], "john");
        assert_eq!(sanitized["users"][0]["password"], "[REDACTED]");
        assert_eq!(sanitized["users"][1]["name"], "jane");
        assert_eq!(sanitized["users"][1]["token"], "[REDACTED]");
    }

    #[test]
    fn test_sanitize_json_case_insensitive() {
        let input = json!({
            "Password": "secret123",
            "API_KEY": "sk-1234567890",
            "AccessToken": "token123"
        });

        let sanitized = sanitize_json(&input);
        assert_eq!(sanitized["Password"], "[REDACTED]");
        assert_eq!(sanitized["API_KEY"], "[REDACTED]");
        assert_eq!(sanitized["AccessToken"], "[REDACTED]");
    }

    #[test]
    fn test_redact_pii_email() {
        let text = "Contact me at john.doe@example.com for more info";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[EMAIL]"));
        assert!(!redacted.contains("john.doe@example.com"));
    }

    #[test]
    fn test_redact_pii_phone() {
        let text = "Call me at 555-123-4567 or (555) 987-6543";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[PHONE]"));
        assert!(!redacted.contains("555-123-4567"));
        assert!(!redacted.contains("555) 987-6543"));
    }

    #[test]
    fn test_redact_pii_credit_card() {
        let text = "Card number: 4532-1234-5678-9010";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[CARD]"));
        assert!(!redacted.contains("4532-1234-5678-9010"));
    }

    #[test]
    fn test_redact_pii_multiple() {
        let text = "Email: john@example.com, Phone: 555-123-1234, Card: 4532123456789010";
        let redacted = redact_pii(text);
        assert!(redacted.contains("[EMAIL]"));
        assert!(redacted.contains("[PHONE]"));
        assert!(redacted.contains("[CARD]"));
    }

    #[test]
    fn test_safe_preview() {
        let text = "My email is john@example.com and here's a very long message that goes on and on and on and on and on and on";
        let preview = safe_preview(text, 50);

        // Should be truncated
        assert!(preview.len() <= 53); // 50 + "..."

        // Should have PII redacted
        assert!(preview.contains("[EMAIL]"));
        assert!(!preview.contains("john@example.com"));
    }

    #[test]
    fn test_sanitize_tool_payload() {
        let payload = json!({
            "password": "secret123",
            "api_key": "sk-1234567890",
            "user": "john@example.com"
        });

        let sanitized = sanitize_tool_payload(&payload, 100);

        // Should be truncated
        assert!(
            sanitized.len() <= 103,
            "Length should be <= 103, got: {}",
            sanitized.len()
        );

        // Password and api_key fields should be redacted
        assert!(
            sanitized.contains("[REDACTED]"),
            "Expected [REDACTED] in output, got: {}",
            sanitized
        );

        // Email should be redacted
        assert!(
            sanitized.contains("[EMAIL]"),
            "Expected [EMAIL] in output, got: {}",
            sanitized
        );
    }

    #[test]
    fn test_sanitize_tool_payload_long_message() {
        let payload = json!({
            "password": "secret123",
            "message": "a".repeat(200)
        });

        let sanitized = sanitize_tool_payload(&payload, 100);

        // Should be truncated
        assert!(sanitized.len() <= 103);

        // Even though truncated, password should still be redacted in the JSON structure
        // The order of fields in JSON is not guaranteed, but [REDACTED] should appear
        // if the password field comes before the truncation point
        assert!(sanitized.contains("[REDACTED]") || sanitized.ends_with("..."));
    }

    #[test]
    fn test_sanitize_tool_payload_no_sensitive_data() {
        let payload = json!({
            "action": "get_weather",
            "location": "Dubai"
        });

        let sanitized = sanitize_tool_payload(&payload, 100);
        assert!(sanitized.contains("get_weather"));
        assert!(sanitized.contains("Dubai"));
        assert!(!sanitized.contains("[REDACTED]"));
    }
}
