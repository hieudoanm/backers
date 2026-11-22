//! Edge-functions client — `/api/functions`.
//!
//! Management (list/upsert/remove) requires an admin token; [`invoke`]
//! is open and uses the client's current Bearer token, if any, as the
//! function's `auth` context.

use serde_json::{Value, json};

use crate::error::Result;
use crate::http::{HttpClient, json_body};
use crate::records::url_encode;
use crate::types::EdgeFunction;

/// Edge functions client.
#[derive(Debug, Clone)]
pub struct FunctionsClient {
    http: HttpClient,
}

impl FunctionsClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        FunctionsClient { http }
    }

    /// Lists every stored function.
    pub async fn list(&self) -> Result<Vec<EdgeFunction>> {
        self.http.get("/api/functions").await
    }

    /// Creates or replaces a function by name.
    pub async fn upsert(&self, name: &str, code: &str) -> Result<EdgeFunction> {
        self.http
            .post(
                "/api/functions",
                json_body(json!({ "name": name, "code": code })),
            )
            .await
    }

    /// Deletes a function by name.
    pub async fn remove(&self, name: &str) -> Result<()> {
        self.http
            .delete(&format!("/api/functions/{}", url_encode(name)))
            .await
    }

    /// Invokes `name` with `input`; resolves to the handler's return value.
    /// The handler receives `{ input, auth }` — `auth` is populated when the
    /// client holds a session token.
    pub async fn invoke<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        input: Option<Value>,
    ) -> Result<T> {
        let res: Value = self
            .http
            .post(
                &format!("/api/functions/{}", url_encode(name)),
                json_body(json!({ "input": input })),
            )
            .await?;
        serde_json::from_value(res.get("result").cloned().unwrap_or(Value::Null))
            .map_err(|e| crate::error::BackersError::Codec(format!("decode result: {e}")))
    }
}
