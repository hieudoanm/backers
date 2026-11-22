use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A stored feature flag.
///
/// `enabled` is the master switch; `rules` are targeting conditions that must
/// **all** match for the flag to evaluate as on. `payload` is arbitrary JSON
/// returned to callers when the flag is on (variant config, rollout %, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub name: String,
    pub enabled: bool,
    pub rules: Vec<FlagRule>,
    pub payload: Value,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// One targeting condition. `attribute` is looked up in the evaluation
/// context (auth claims like `user_id`/`email`, plus caller-supplied context);
/// `op` is one of `eq`, `neq`, `in`, `contains`, `starts_with`, `ends_with`,
/// `gt`, `gte`, `lt`, `lte`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagRule {
    pub attribute: String,
    pub op: String,
    pub value: Value,
}

/// Body for `POST /api/flags` — creates or replaces a flag.
#[derive(Debug, Deserialize)]
pub struct UpsertFlagRequest {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<FlagRule>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub description: String,
}

/// Body for `PATCH /api/flags/{name}` — every field optional.
#[derive(Debug, Deserialize)]
pub struct PatchFlagRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub rules: Option<Vec<FlagRule>>,
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Body for `POST /api/flags/evaluate`.
#[derive(Debug, Deserialize)]
pub struct EvaluateFlagsRequest {
    /// Flag names to evaluate; empty means every flag.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Extra attributes merged into the evaluation context
    /// (e.g. `{"plan": "pro", "country": "VN"}`).
    #[serde(default)]
    pub context: serde_json::Map<String, Value>,
}

/// The evaluation result for one flag.
#[derive(Debug, Clone, Serialize)]
pub struct FlagEvaluation {
    pub enabled: bool,
    /// Present only when the flag evaluated as on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}
