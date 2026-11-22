//! Feature-flags client — `/api/flags`.
//!
//! Management (`list`/`get`/`upsert`/`update`/`remove`) requires an admin
//! token; [`evaluate`](Self::evaluate) is open and uses the client's Bearer
//! token, if any, to expose `user_id`/`email` to rules.

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::http::{HttpClient, json_body, json_of};
use crate::records::url_encode;
use crate::types::{Flag, FlagEvaluation, FlagPatch, FlagUpsert};

/// Feature flags client.
#[derive(Debug, Clone)]
pub struct FlagsClient {
    http: HttpClient,
}

impl FlagsClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        FlagsClient { http }
    }

    /// Lists all flags, or a subset when `names` is non-empty.
    pub async fn list(&self, names: &[String]) -> Result<Vec<Flag>> {
        let path = if names.is_empty() {
            "/api/flags".to_string()
        } else {
            format!("/api/flags?names={}", url_encode(&names.join(",")))
        };
        self.http.get(&path).await
    }

    /// Fetches one flag by name.
    pub async fn get(&self, name: &str) -> Result<Flag> {
        self.http
            .get(&format!("/api/flags/{}", url_encode(name)))
            .await
    }

    /// Creates or replaces a flag by name.
    pub async fn upsert(&self, flag: &FlagUpsert) -> Result<Flag> {
        self.http.post("/api/flags", json_of(flag)?).await
    }

    /// Partially updates a flag (`enabled`, `rules`, `payload`, `description`).
    pub async fn update(&self, name: &str, patch: &FlagPatch) -> Result<Flag> {
        self.http
            .patch(&format!("/api/flags/{}", url_encode(name)), json_of(patch)?)
            .await
    }

    /// Deletes a flag by name.
    pub async fn remove(&self, name: &str) -> Result<()> {
        self.http
            .delete(&format!("/api/flags/{}", url_encode(name)))
            .await
    }

    /// Evaluates flags for the current context. An empty `names` slice
    /// evaluates every flag; `context` adds attributes (e.g. `plan`,
    /// `country`) that rules can match. Returns a name → evaluation map.
    pub async fn evaluate(
        &self,
        names: &[String],
        context: &HashMap<String, Value>,
    ) -> Result<HashMap<String, FlagEvaluation>> {
        let mut ctx = Map::new();
        for (k, v) in context {
            ctx.insert(k.clone(), v.clone());
        }
        let body = json!({ "flags": names, "context": Value::Object(ctx) });
        #[derive(serde::Deserialize)]
        struct EvalResponse {
            flags: HashMap<String, FlagEvaluation>,
        }
        let res: EvalResponse = self
            .http
            .post("/api/flags/evaluate", json_body(body))
            .await?;
        Ok(res.flags)
    }
}
