//! Feature flags: admin CRUD over `_flags` plus a public evaluation endpoint.
//!
//! Evaluation is open (like function invocation) — send a Bearer token to add
//! `user_id`/`email` to the attribute context, and pass extra targeting
//! attributes in the request `context`. Mutations broadcast a `_flags`
//! record-change event on the WebSocket hub so subscribed clients can refresh.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;

async fn conn(
    state: &AppState,
) -> std::result::Result<deadpool::managed::Object<db::ConnectionManager>, AppError> {
    state
        .db
        .get()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// GET /api/flags — list all flags, or `?names=a,b` for a subset (admin).
pub async fn list_flags(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Query(query): Query<ListFlagsQuery>,
) -> std::result::Result<Json<Vec<Flag>>, AppError> {
    rbac::require_role_pub(&state, &headers, "*", "admin").await?;
    let conn = conn(&state).await?;
    let names: Vec<String> = query
        .names
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    Ok(Json(db::list_flags(&conn, &names)?))
}

#[derive(Debug, Deserialize)]
pub struct ListFlagsQuery {
    names: Option<String>,
}

/// GET /api/flags/{name} — one flag (admin).
pub async fn get_flag(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<Json<Flag>, AppError> {
    rbac::require_role_pub(&state, &headers, "*", "admin").await?;
    let conn = conn(&state).await?;
    db::get_flag(&conn, &name)?
        .map(Json)
        .ok_or_else(|| AppError::NotFound("flag not found".into()))
}

/// POST /api/flags — create or replace a flag (admin).
pub async fn upsert_flag(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Json(req): Json<UpsertFlagRequest>,
) -> std::result::Result<Json<Flag>, AppError> {
    rbac::require_role_pub(&state, &headers, "*", "admin").await?;
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let conn = conn(&state).await?;
    let flag = db::upsert_flag(&conn, &req)?;
    broadcast_flag(&state, "upsert", &flag).await;
    Ok(Json(flag))
}

/// PATCH /api/flags/{name} — partial update (admin).
pub async fn patch_flag(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<PatchFlagRequest>,
) -> std::result::Result<Json<Flag>, AppError> {
    rbac::require_role_pub(&state, &headers, "*", "admin").await?;
    let conn = conn(&state).await?;
    let flag = db::patch_flag(&conn, &name, &req)?;
    broadcast_flag(&state, "update", &flag).await;
    Ok(Json(flag))
}

/// DELETE /api/flags/{name} — remove a flag (admin).
pub async fn delete_flag(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Path(name): Path<String>,
) -> std::result::Result<StatusCode, AppError> {
    rbac::require_role_pub(&state, &headers, "*", "admin").await?;
    let conn = conn(&state).await?;
    if !db::delete_flag(&conn, &name)? {
        return Err(AppError::NotFound("flag not found".into()));
    }
    let record = json!({ "name": name });
    let _ = websocket::broadcast_record(&state.ws_hub, "_flags", "delete", &record).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/flags/evaluate — resolve flags for the caller.
///
/// Body: `{ "flags": ["new-ui"], "context": {"plan": "pro"} }`. `flags`
/// empty evaluates every flag. Auth is optional: a Bearer token contributes
/// `user_id` and `email` attributes (which win over same-named context keys).
/// Response: `{ "flags": { "<name>": { "enabled": bool, "payload"?: any } } }`.
pub async fn evaluate_flags(
    headers: HeaderMap,
    State(state): State<std::sync::Arc<AppState>>,
    Json(req): Json<EvaluateFlagsRequest>,
) -> std::result::Result<Json<Value>, AppError> {
    let conn = conn(&state).await?;

    let mut attrs: HashMap<String, Value> = HashMap::new();
    for (key, value) in &req.context {
        attrs.insert(key.clone(), value.clone());
    }
    if let Ok(claims) = extract_claims(&headers) {
        attrs.insert("user_id".into(), json!(claims.user_id));
        attrs.insert("email".into(), json!(claims.email));
    }

    let flags = db::evaluate_flags(&conn, &req.flags, &attrs)?;
    Ok(Json(json!({ "flags": Value::Object(flags) })))
}

/// Emits a `_flags` record-change event so WS subscribers refresh flag state.
async fn broadcast_flag(state: &AppState, action: &str, flag: &Flag) {
    if let Ok(record) = serde_json::to_value(flag) {
        let _ = websocket::broadcast_record(&state.ws_hub, "_flags", action, &record).await;
    }
}
