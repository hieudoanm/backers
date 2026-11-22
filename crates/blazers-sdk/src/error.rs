//! Error type thrown for non-2xx API responses — mirrors `BlazersError` in
//! the TypeScript SDK.

use serde_json::Value;

/// Error thrown for non-2xx API responses or transport failures.
#[derive(Debug, thiserror::Error)]
pub enum BlazersError {
    /// Non-2xx HTTP response. `body` is the parsed JSON error body when
    /// available, otherwise the raw text.
    #[error("{message}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Human-readable detail extracted from the body.
        message: String,
        /// Parsed JSON error body when available, otherwise raw text.
        body: Value,
    },
    /// Network/transport failure (DNS, TLS, connection reset, …).
    #[error("request failed: {0}")]
    Transport(String),
    /// Response or request payload could not be (de)serialized.
    #[error("codec error: {0}")]
    Codec(String),
}

impl BlazersError {
    /// HTTP status code, or `0` for transport/codec failures.
    pub fn status(&self) -> u16 {
        match self {
            BlazersError::Http { status, .. } => *status,
            _ => 0,
        }
    }

    /// The response body for `Http` errors.
    pub fn body(&self) -> Option<&Value> {
        match self {
            BlazersError::Http { body, .. } => Some(body),
            _ => None,
        }
    }

    /// Builds an `Http` error from a status and raw response text, extracting
    /// `error` from a JSON body when present (matches `AppError`'s shape).
    pub(crate) fn from_response(status: u16, text: &str) -> Self {
        let parsed: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        let message = parsed
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                }
            })
            .unwrap_or_else(|| format!("HTTP {status}"));
        BlazersError::Http {
            status,
            message,
            body: if parsed.is_null() {
                Value::String(text.to_string())
            } else {
                parsed
            },
        }
    }
}

/// Convenience alias used across sub-clients.
pub type Result<T> = std::result::Result<T, BlazersError>;
