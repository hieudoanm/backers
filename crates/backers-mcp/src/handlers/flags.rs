//! `backers_flags_*` tool handlers — feature-flag CRUD and evaluation against
//! the same `_flags` table and rule engine the HTTP API uses.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::*;
use backers_core::models::{FlagRule, UpsertFlagRequest};

/// Arguments of `backers_flags_list`.
#[derive(Debug, Deserialize)]
pub struct FlagsListArgs {
    /// Optional subset of flag names; empty lists every flag.
    #[serde(default)]
    pub names: Vec<String>,
}

/// Arguments of `backers_flags_get` and `backers_flags_delete`.
#[derive(Debug, Deserialize)]
pub struct FlagNameArgs {
    pub name: String,
}

/// Arguments of `backers_flags_upsert`.
#[derive(Debug, Deserialize)]
pub struct FlagUpsertArgs {
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

/// Arguments of `backers_flags_evaluate`.
#[derive(Debug, Deserialize)]
pub struct FlagEvaluateArgs {
    /// Flag names to evaluate; empty evaluates every flag.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Extra attributes the rules match against
    /// (e.g. `{"plan": "pro", "country": "VN"}`).
    #[serde(default)]
    pub context: serde_json::Map<String, Value>,
}

/// `backers_flags_list` returns flags, optionally filtered by name.
pub fn handle_flags_list(db: &DbHandle, arguments: &str) -> ToolResult {
    let args: FlagsListArgs = match parse_args(arguments) {
        Ok(args) => args,
        Err(message) => return error_result(message),
    };
    match with_db(db, |conn| db::list_flags(conn, &args.names)) {
        Ok(flags) => json_result(&json!({ "flags": flags })),
        Err(message) => error_result(format!("list flags: {message}")),
    }
}

/// `backers_flags_get` returns one flag by name.
pub fn handle_flags_get(db: &DbHandle, arguments: &str) -> ToolResult {
    let args: FlagNameArgs = match parse_args(arguments) {
        Ok(args) => args,
        Err(message) => return error_result(message),
    };
    match with_db(db, |conn| db::get_flag(conn, &args.name)) {
        Ok(Some(flag)) => json_result(&serde_json::to_value(flag).unwrap_or(Value::Null)),
        Ok(None) => error_result(format!("flag '{}' not found", args.name)),
        Err(message) => error_result(format!("get flag: {message}")),
    }
}

/// `backers_flags_upsert` creates or replaces a flag by name.
pub fn handle_flags_upsert(db: &DbHandle, arguments: &str) -> ToolResult {
    let args: FlagUpsertArgs = match parse_args(arguments) {
        Ok(args) => args,
        Err(message) => return error_result(message),
    };
    if args.name.trim().is_empty() {
        return error_result("flag name must not be empty");
    }
    let req = UpsertFlagRequest {
        name: args.name,
        enabled: args.enabled,
        rules: args.rules,
        payload: args.payload,
        description: args.description,
    };
    match with_db(db, |conn| db::upsert_flag(conn, &req)) {
        Ok(flag) => json_result(&serde_json::to_value(flag).unwrap_or(Value::Null)),
        Err(message) => error_result(format!("upsert flag: {message}")),
    }
}

/// `backers_flags_delete` removes one flag by name.
pub fn handle_flags_delete(db: &DbHandle, arguments: &str) -> ToolResult {
    let args: FlagNameArgs = match parse_args(arguments) {
        Ok(args) => args,
        Err(message) => return error_result(message),
    };
    match with_db(db, |conn| db::delete_flag(conn, &args.name)) {
        Ok(true) => json_result(&json!({ "name": args.name, "deleted": true })),
        Ok(false) => error_result(format!("flag '{}' not found", args.name)),
        Err(message) => error_result(format!("delete flag: {message}")),
    }
}

/// `backers_flags_evaluate` resolves flags against a caller-supplied context.
/// MCP has no bearer token, so `user_id`/`email` attributes are unavailable —
/// pass them through `context` when a rule needs them.
pub fn handle_flags_evaluate(db: &DbHandle, arguments: &str) -> ToolResult {
    let args: FlagEvaluateArgs = match parse_args(arguments) {
        Ok(args) => args,
        Err(message) => return error_result(message),
    };
    let attrs: HashMap<String, Value> = args.context.into_iter().collect();
    match with_db(db, |conn| db::evaluate_flags(conn, &args.flags, &attrs)) {
        Ok(flags) => json_result(&json!({ "flags": Value::Object(flags) })),
        Err(message) => error_result(format!("evaluate flags: {message}")),
    }
}
