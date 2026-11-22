//! Low-level HTTP transport shared by all sub-clients.
//!
//! Two backends behind one API: `reqwest` on native targets and `web-sys`
//! `fetch` on `wasm32` (Node ≥22 / browsers), so the whole crate compiles to
//! wasm for JS usage.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{BackersError, Result};
use crate::types::{BatchResult, BatchSubRequest};

#[cfg(not(target_arch = "wasm32"))]
mod transport_native;
#[cfg(target_arch = "wasm32")]
mod transport_wasm;
#[cfg(not(target_arch = "wasm32"))]
use transport_native as transport;
#[cfg(target_arch = "wasm32")]
use transport_wasm as transport;

/// Construction options for [`HttpClient`].
#[derive(Debug, Clone, Default)]
pub struct ClientOptions {
    /// Base URL of the Backers server, e.g. `http://localhost:8090`.
    pub url: String,
    /// Bearer token attached to every request.
    pub token: Option<String>,
    /// Extra headers sent with every request.
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub(crate) enum MultipartField {
    Text {
        name: String,
        value: String,
    },
    File {
        name: String,
        filename: String,
        data: Vec<u8>,
        content_type: Option<String>,
    },
}

/// A `multipart/form-data` body builder (file uploads).
#[derive(Debug, Default)]
pub struct Multipart {
    pub(crate) fields: Vec<MultipartField>,
}

impl Multipart {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a text field.
    pub fn text_field(mut self, name: &str, value: &str) -> Self {
        self.fields.push(MultipartField::Text {
            name: name.to_string(),
            value: value.to_string(),
        });
        self
    }

    /// Adds a file field from raw bytes.
    pub fn file_field(
        mut self,
        name: &str,
        filename: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Self {
        self.fields.push(MultipartField::File {
            name: name.to_string(),
            filename: filename.to_string(),
            data,
            content_type: content_type.map(str::to_string),
        });
        self
    }
}

/// Request payload: JSON or multipart form.
#[derive(Debug)]
pub enum Body {
    /// JSON-encoded value; sets `Content-Type: application/json`.
    Json(Value),
    /// Multipart form fields (file uploads).
    Multipart(Multipart),
}

#[derive(Debug)]
pub(crate) struct Inner {
    pub base_url: String,
    pub token: RwLock<Option<String>>,
    pub headers: HashMap<String, String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub client: reqwest::Client,
}

/// Low-level transport shared by all sub-clients. Cheap to clone.
#[derive(Debug, Clone)]
pub struct HttpClient {
    pub(crate) inner: Arc<Inner>,
}

impl HttpClient {
    pub fn new(options: ClientOptions) -> Self {
        let base_url = options.url.trim_end_matches('/').to_string();
        #[cfg(not(target_arch = "wasm32"))]
        let client = reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        HttpClient {
            inner: Arc::new(Inner {
                base_url,
                token: RwLock::new(options.token),
                headers: options.headers,
                #[cfg(not(target_arch = "wasm32"))]
                client,
            }),
        }
    }

    /// Current bearer token, if any.
    pub fn auth_token(&self) -> Option<String> {
        self.inner.token.read().ok().and_then(|t| t.clone())
    }

    /// Sets or clears (`None`) the session token.
    pub fn set_token(&self, token: Option<String>) {
        if let Ok(mut guard) = self.inner.token.write() {
            *guard = token;
        }
    }

    /// Absolute URL for an API path.
    /// Base URL the client was configured with (no trailing slash).
    pub fn base_url(&self) -> String {
        self.inner.base_url.clone()
    }

    pub fn url(&self, path: &str) -> String {
        let path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        format!("{}{path}", self.inner.base_url)
    }

    /// WebSocket URL for `/ws`.
    pub fn ws_url(&self) -> String {
        let base = &self.inner.base_url;
        let scheme = if base.starts_with("https://") {
            "wss://"
        } else {
            "ws://"
        };
        let rest = base
            .strip_prefix("https://")
            .or_else(|| base.strip_prefix("http://"))
            .unwrap_or(base);
        format!("{scheme}{rest}/ws")
    }

    /// Sends one request and returns the parsed JSON body (`Null` for empty).
    pub(crate) async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Body>,
        extra_headers: &HashMap<String, String>,
    ) -> Result<Value> {
        let mut headers: Vec<(String, String)> = self
            .inner
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        headers.extend(extra_headers.iter().map(|(k, v)| (k.clone(), v.clone())));
        if let Some(token) = self.auth_token() {
            headers.push(("authorization".into(), format!("Bearer {token}")));
        }
        if matches!(body, Some(Body::Json(_)))
            && !headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            headers.push(("content-type".into(), "application/json".into()));
        }

        let (status, text) =
            transport::send(&self.inner, method, &self.url(path), headers, body).await?;
        if !(200..300).contains(&status) {
            return Err(BackersError::from_response(status, &text));
        }
        if text.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).map_err(|e| BackersError::Codec(format!("parse response: {e}")))
    }

    /// Typed JSON request — deserializes the response body into `T`.
    pub async fn request_typed<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<Body>,
    ) -> Result<T> {
        let value = self.request(method, path, body, &HashMap::new()).await?;
        serde_json::from_value(value)
            .map_err(|e| BackersError::Codec(format!("decode response: {e}")))
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_typed("GET", path, None).await
    }

    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: Option<Body>) -> Result<T> {
        self.request_typed("POST", path, body).await
    }

    pub async fn patch<T: DeserializeOwned>(&self, path: &str, body: Option<Body>) -> Result<T> {
        self.request_typed("PATCH", path, body).await
    }

    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: Option<Body>) -> Result<T> {
        self.request_typed("PUT", path, body).await
    }

    /// DELETE — discards the (usually empty) response body.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let _: Value = self.request("DELETE", path, None, &HashMap::new()).await?;
        Ok(())
    }

    /// `POST /api/batch` — up to 50 sub-requests in one round-trip.
    pub async fn batch(&self, requests: Vec<BatchSubRequest>) -> Result<Vec<BatchResult>> {
        let body = serde_json::json!({ "requests": requests });
        self.post("/api/batch", Some(Body::Json(body))).await
    }
}

/// JSON request body shorthand.
pub(crate) fn json_body(value: Value) -> Option<Body> {
    Some(Body::Json(value))
}

/// Serializes `value` into a JSON request body.
pub(crate) fn json_of<T: serde::Serialize>(value: &T) -> Result<Option<Body>> {
    serde_json::to_value(value)
        .map(|v| Some(Body::Json(v)))
        .map_err(|e| BackersError::Codec(format!("encode request: {e}")))
}
