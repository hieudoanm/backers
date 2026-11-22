//! Feature-flag storage and evaluation.
//!
//! Flags live in `_flags`: a master `enabled` switch, JSON `rules` (all must
//! match), and a JSON `payload` returned when the flag is on. Evaluation is a
//! pure function of the flag row plus an attribute map built from the caller's
//! auth claims and request context.

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::models::*;

const FLAG_COLS: &str = "name, enabled, rules, payload, description, created_at, updated_at";

fn row_to_flag(row: &rusqlite::Row) -> rusqlite::Result<Flag> {
    let rules_json: String = row.get(2)?;
    let payload_json: String = row.get(3)?;
    Ok(Flag {
        name: row.get(0)?,
        enabled: row.get::<_, i64>(1)? != 0,
        rules: serde_json::from_str(&rules_json).unwrap_or_default(),
        payload: serde_json::from_str(&payload_json).unwrap_or(Value::Null),
        description: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

/// Creates or replaces a flag by name.
pub fn upsert_flag(conn: &Connection, req: &UpsertFlagRequest) -> Result<Flag> {
    let now = Utc::now().to_rfc3339();
    let rules = serde_json::to_string(&req.rules).unwrap_or_else(|_| "[]".into());
    let payload = serde_json::to_string(&req.payload).unwrap_or_else(|_| "null".into());
    conn.execute(
        "INSERT INTO _flags (name, enabled, rules, payload, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT(name) DO UPDATE SET
             enabled = ?2, rules = ?3, payload = ?4, description = ?5, updated_at = ?6",
        params![
            req.name,
            req.enabled as i64,
            rules,
            payload,
            req.description,
            now
        ],
    )
    .map_err(|e| AppError::Internal(format!("upsert flag: {e}")))?;
    get_flag(conn, &req.name)?
        .ok_or_else(|| AppError::Internal("flag not found after upsert".into()))
}

/// Applies a partial update; `NotFound` when the flag does not exist.
pub fn patch_flag(conn: &Connection, name: &str, req: &PatchFlagRequest) -> Result<Flag> {
    let existing = get_flag(conn, name)?.ok_or(AppError::NotFound("flag not found".into()))?;
    let enabled = req.enabled.unwrap_or(existing.enabled);
    let rules = req.rules.clone().unwrap_or(existing.rules);
    let payload = req.payload.clone().unwrap_or(existing.payload);
    let description = req.description.clone().unwrap_or(existing.description);
    let now = Utc::now().to_rfc3339();
    let rules_json = serde_json::to_string(&rules).unwrap_or_else(|_| "[]".into());
    let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "null".into());
    conn.execute(
        "UPDATE _flags SET enabled = ?2, rules = ?3, payload = ?4, description = ?5, updated_at = ?6
         WHERE name = ?1",
        params![
            name,
            enabled as i64,
            rules_json,
            payload_json,
            description,
            now
        ],
    )
    .map_err(|e| AppError::Internal(format!("patch flag: {e}")))?;
    get_flag(conn, name)?.ok_or_else(|| AppError::Internal("flag lost after patch".into()))
}

pub fn get_flag(conn: &Connection, name: &str) -> Result<Option<Flag>> {
    let mut stmt = conn
        .prepare(&format!("SELECT {FLAG_COLS} FROM _flags WHERE name = ?1"))
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mut rows = stmt
        .query_map(params![name], row_to_flag)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    match rows.next() {
        Some(Ok(f)) => Ok(Some(f)),
        Some(Err(e)) => Err(AppError::Internal(e.to_string())),
        None => Ok(None),
    }
}

/// Lists every flag, or only `names` when non-empty (unknown names skipped).
pub fn list_flags(conn: &Connection, names: &[String]) -> Result<Vec<Flag>> {
    if names.is_empty() {
        let mut stmt = conn
            .prepare(&format!("SELECT {FLAG_COLS} FROM _flags ORDER BY name"))
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_flag)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        return rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::Internal(e.to_string()));
    }
    let mut flags = Vec::with_capacity(names.len());
    for name in names {
        if let Some(flag) = get_flag(conn, name)? {
            flags.push(flag);
        }
    }
    Ok(flags)
}

pub fn delete_flag(conn: &Connection, name: &str) -> Result<bool> {
    let n = conn
        .execute("DELETE FROM _flags WHERE name = ?1", params![name])
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(n > 0)
}

/// Evaluates one flag against `attrs`. Returns `None` for unknown flags.
///
/// A flag is on when `enabled` is set **and** every rule matches. Rules look
/// up `attribute` in `attrs`; a missing attribute never matches (except `neq`,
/// which treats absence as unequal).
pub fn evaluate_flag(
    conn: &Connection,
    name: &str,
    attrs: &HashMap<String, Value>,
) -> Result<Option<FlagEvaluation>> {
    let Some(flag) = get_flag(conn, name)? else {
        return Ok(None);
    };
    Ok(Some(evaluate(&flag, attrs)))
}

/// Evaluates every requested flag (or all flags when `names` is empty)
/// against `attrs`, returning a name → result map. Unknown names are omitted.
pub fn evaluate_flags(
    conn: &Connection,
    names: &[String],
    attrs: &HashMap<String, Value>,
) -> Result<Map<String, Value>> {
    let mut out = Map::new();
    for flag in list_flags(conn, names)? {
        let evaluation = evaluate(&flag, attrs);
        out.insert(
            flag.name.clone(),
            serde_json::to_value(evaluation).unwrap_or(Value::Null),
        );
    }
    Ok(out)
}

fn evaluate(flag: &Flag, attrs: &HashMap<String, Value>) -> FlagEvaluation {
    let on = flag.enabled && flag.rules.iter().all(|rule| rule_matches(rule, attrs));
    FlagEvaluation {
        enabled: on,
        payload: on.then(|| flag.payload.clone()),
    }
}

fn rule_matches(rule: &FlagRule, attrs: &HashMap<String, Value>) -> bool {
    let actual = attrs.get(&rule.attribute);
    match rule.op.as_str() {
        "eq" => actual.is_some_and(|a| values_equal(a, &rule.value)),
        "neq" => !actual.is_some_and(|a| values_equal(a, &rule.value)),
        "in" => actual.is_some_and(|a| {
            rule.value
                .as_array()
                .is_some_and(|list| list.iter().any(|v| values_equal(a, v)))
        }),
        "contains" => match (actual, &rule.value) {
            (Some(Value::String(a)), Value::String(b)) => a.contains(b.as_str()),
            (Some(Value::Array(a)), b) => a.iter().any(|v| values_equal(v, b)),
            _ => false,
        },
        "starts_with" => match (actual, &rule.value) {
            (Some(Value::String(a)), Value::String(b)) => a.starts_with(b.as_str()),
            _ => false,
        },
        "ends_with" => match (actual, &rule.value) {
            (Some(Value::String(a)), Value::String(b)) => a.ends_with(b.as_str()),
            _ => false,
        },
        "gt" | "gte" | "lt" | "lte" => match (actual.and_then(as_number), as_number(&rule.value)) {
            (Some(a), Some(b)) => match rule.op.as_str() {
                "gt" => a > b,
                "gte" => a >= b,
                "lt" => a < b,
                _ => a <= b,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Loose equality: numbers compare numerically (JSON strings that parse as
/// numbers included), everything else compares as JSON values.
fn values_equal(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
        return x == y;
    }
    a == b
}

/// Extracts an f64 from a JSON number or a numeric string.
fn as_number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
}
