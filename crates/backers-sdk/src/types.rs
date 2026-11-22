//! Wire types mirroring the Backers REST API (`/api/*`) and realtime frames.
//!
//! Field names match the TypeScript SDK exactly — snake_case on the wire.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An authenticated user (`/api/auth/*`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    /// Custom user fields (schemaless JSON).
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub mfa_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// `{ user, token }` returned by login/refresh/OTP/anonymous flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub user: User,
    pub token: String,
}

/// `POST /api/auth/login` returns either a full session or an MFA challenge.
/// Untagged: a response carrying `token` is a session; `{mfa_required, user}`
/// without a token is the challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoginResult {
    Session(LoginResponse),
    MfaChallenge { mfa_required: bool, user: User },
}

impl LoginResult {
    /// The session token when this is a completed login.
    pub fn token(&self) -> Option<&str> {
        match self {
            LoginResult::Session(s) => Some(&s.token),
            LoginResult::MfaChallenge { .. } => None,
        }
    }
    /// The user in either variant.
    pub fn user(&self) -> &User {
        match self {
            LoginResult::Session(s) => &s.user,
            LoginResult::MfaChallenge { user, .. } => user,
        }
    }
}

/// A stored record: `{ id, data, created_at, updated_at }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordData {
    pub id: String,
    /// Schemaless document; `expand_<field>` keys appear after `?expand=`.
    #[serde(default)]
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
}

/// One page of records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordsPage {
    #[serde(default)]
    pub records: Vec<RecordData>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub page: i64,
    #[serde(default, rename = "per_page")]
    pub per_page: i64,
    #[serde(default, rename = "total_pages")]
    pub total_pages: i64,
}

/// Query options for record listing (`?page&per_page&search&filter&sort&expand`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "per_page",
        alias = "perPage"
    )]
    pub per_page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    /// PocketBase-style filter expression, e.g. `status="live" && views>10`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    /// Comma-separated fields; prefix with `-` for descending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Comma-separated relation fields to resolve into `data.expand_<field>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

/// The five PocketBase-style rule expressions on a collection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollectionRules {
    #[serde(skip_serializing_if = "Option::is_none", rename = "list_rule")]
    pub list_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "view_rule")]
    pub view_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "create_rule")]
    pub create_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "update_rule")]
    pub update_rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "delete_rule")]
    pub delete_rule: Option<String>,
}

/// A collection (`/api/collections*`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    /// JSON-schema string validating `record.data`.
    #[serde(default)]
    pub schema: String,
    #[serde(default, rename = "list_rule")]
    pub list_rule: String,
    #[serde(default, rename = "view_rule")]
    pub view_rule: String,
    #[serde(default, rename = "create_rule")]
    pub create_rule: String,
    #[serde(default, rename = "update_rule")]
    pub update_rule: String,
    #[serde(default, rename = "delete_rule")]
    pub delete_rule: String,
    /// "records" for data collections, "auth" for the managed users collection.
    #[serde(default)]
    pub r#type: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A file bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    #[serde(default, rename = "is_public")]
    pub is_public: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A stored file record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub bucket: String,
    pub filename: String,
    #[serde(default, rename = "mime_type")]
    pub mime_type: String,
    #[serde(default)]
    pub size: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// One page of files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesPage {
    #[serde(default)]
    pub files: Vec<FileRecord>,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub page: i64,
    #[serde(default, rename = "per_page")]
    pub per_page: i64,
    #[serde(default, rename = "total_pages")]
    pub total_pages: i64,
}

/// One sub-request inside `POST /api/batch`.
#[derive(Debug, Clone, Serialize)]
pub struct BatchSubRequest {
    pub method: String,
    /// Path or absolute URL under the same origin.
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_json::Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// Per-request result inside a batch response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub status: u16,
    #[serde(default)]
    pub body: Value,
}

/// A short-lived download token for a protected file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileToken {
    pub token: String,
    #[serde(default, rename = "expires_in")]
    pub expires_in: i64,
}

// ---- Feature flags ----

/// Rule operators supported by flag targeting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlagOp {
    Eq,
    Neq,
    In,
    Contains,
    StartsWith,
    EndsWith,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// One targeting condition; all rules must match (AND).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagRule {
    /// Attribute looked up in the evaluation context.
    pub attribute: String,
    pub op: FlagOp,
    pub value: Value,
}

/// A stored feature flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<FlagRule>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `POST /api/flags` — creates or replaces a flag by name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlagUpsert {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<FlagRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Body for `PATCH /api/flags/{name}` — every field optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlagPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<FlagRule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Evaluation result for one flag; `payload` is present only when on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagEvaluation {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

// ---- Edge functions ----

/// A stored edge function (JS source executed by the embedded runtime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFunction {
    pub name: String,
    pub code: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

// ---- Realtime (WebSocket `/ws`) ----

/// Record mutation kind on a realtime event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordAction {
    Create,
    Update,
    Delete,
}

/// A record create/update/delete frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub action: RecordAction,
    pub collection: String,
    pub record: RecordData,
    /// Present on update/delete: the record as it was before the mutation.
    #[serde(default, rename = "old_record", alias = "oldRecord")]
    pub old_record: Option<RecordData>,
}

/// Presence change kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresenceAction {
    Join,
    Leave,
    State,
}

/// A presence frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub action: PresenceAction,
    pub channel: String,
    /// Client id of the member that changed.
    pub client: String,
    #[serde(default)]
    pub state: Option<Value>,
}

/// Any frame the server pushes that is not a record or presence event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

/// Any realtime event delivered to subscribers.
#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    Record(RecordEvent),
    Presence(PresenceEvent),
    Raw(RawEvent),
}

impl RealtimeEvent {
    /// Parses a JSON frame into the right variant by its `type` field.
    pub fn from_value(value: Value) -> Option<Self> {
        let kind = value.get("type")?.as_str()?;
        Some(match kind {
            "record" => Self::Record(serde_json::from_value(value).ok()?),
            "presence" => Self::Presence(serde_json::from_value(value).ok()?),
            _ => Self::Raw(serde_json::from_value(value).ok()?),
        })
    }

    /// Serializes back to the wire JSON frame.
    pub fn to_value(&self) -> Value {
        let v = match self {
            Self::Record(e) => serde_json::to_value(e),
            Self::Presence(e) => serde_json::to_value(e),
            Self::Raw(e) => serde_json::to_value(e),
        };
        v.unwrap_or(Value::Null)
    }
}
