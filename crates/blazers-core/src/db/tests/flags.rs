use std::collections::HashMap;

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::db::*;
use crate::models::*;

fn conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    migrate_db(&conn).unwrap();
    conn
}

fn upsert(conn: &Connection, name: &str, enabled: bool, rules: Value) {
    let req: UpsertFlagRequest = serde_json::from_value(json!({
        "name": name,
        "enabled": enabled,
        "rules": rules,
        "payload": {"v": 1},
    }))
    .unwrap();
    upsert_flag(conn, &req).unwrap();
}

fn attrs(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn disabled_flag_is_off_regardless_of_rules() {
    let conn = conn();
    upsert(&conn, "f", false, json!([]));
    let result = evaluate_flag(&conn, "f", &attrs(&[])).unwrap().unwrap();
    assert!(!result.enabled);
    assert!(result.payload.is_none());
}

#[test]
fn enabled_flag_with_no_rules_is_on_for_everyone() {
    let conn = conn();
    upsert(&conn, "f", true, json!([]));
    let result = evaluate_flag(&conn, "f", &attrs(&[])).unwrap().unwrap();
    assert!(result.enabled);
    assert_eq!(result.payload, Some(json!({"v": 1})));
}

#[test]
fn all_rules_must_match() {
    let conn = conn();
    upsert(
        &conn,
        "f",
        true,
        json!([
            {"attribute": "plan", "op": "eq", "value": "pro"},
            {"attribute": "country", "op": "eq", "value": "VN"},
        ]),
    );
    let context = attrs(&[("plan", json!("pro")), ("country", json!("US"))]);
    assert!(
        !evaluate_flag(&conn, "f", &context)
            .unwrap()
            .unwrap()
            .enabled
    );
    let context = attrs(&[("plan", json!("pro")), ("country", json!("VN"))]);
    assert!(
        evaluate_flag(&conn, "f", &context)
            .unwrap()
            .unwrap()
            .enabled
    );
}

#[test]
fn rule_operators() {
    let conn = conn();
    let cases: Vec<(&str, &str, Value, Value, bool)> = vec![
        ("eq", "plan", json!("pro"), json!("pro"), true),
        ("eq", "plan", json!("pro"), json!("free"), false),
        ("eq", "n", json!(42), json!("42"), true), // numeric/string loose eq
        ("neq", "plan", json!("pro"), json!("free"), true),
        ("neq", "missing", json!("x"), json!("y"), true), // absent attr ≠ value
        ("in", "plan", json!("pro"), json!(["pro", "team"]), true),
        ("in", "plan", json!("free"), json!(["pro", "team"]), false),
        ("contains", "email", json!("a@b.com"), json!("@b.com"), true),
        ("starts_with", "email", json!("a@b.com"), json!("a@"), true),
        ("ends_with", "email", json!("a@b.com"), json!(".com"), true),
        ("gt", "spent", json!(150), json!(100), true),
        ("gte", "spent", json!(100), json!(100), true),
        ("lt", "spent", json!(150), json!(100), false),
        ("lt", "spent", json!(50), json!(100), true),
        ("lte", "spent", json!(100), json!(100), true),
        ("bogus", "plan", json!("pro"), json!("pro"), false),
    ];
    for (i, (op, attr_name, attr_val, rule_val, expected)) in cases.iter().enumerate() {
        let name = format!("f{i}");
        upsert(
            &conn,
            &name,
            true,
            json!([{"attribute": attr_name, "op": op, "value": rule_val}]),
        );
        let context = attrs(&[(attr_name, attr_val.clone())]);
        let got = evaluate_flag(&conn, &name, &context).unwrap().unwrap();
        assert_eq!(got.enabled, *expected, "{op} failed");
    }
}

#[test]
fn evaluate_flags_maps_names_and_skips_unknown() {
    let conn = conn();
    upsert(&conn, "a", true, json!([]));
    upsert(&conn, "b", false, json!([]));
    let result = evaluate_flags(
        &conn,
        &["a".into(), "b".into(), "ghost".into()],
        &attrs(&[]),
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result["a"]["enabled"], json!(true));
    assert_eq!(result["b"]["enabled"], json!(false));
    assert!(result.get("ghost").is_none());
}

#[test]
fn patch_preserves_unspecified_fields() {
    let conn = conn();
    upsert(&conn, "f", true, json!([]));
    let req: PatchFlagRequest =
        serde_json::from_value(json!({"enabled": false, "description": "off"})).unwrap();
    let flag = patch_flag(&conn, "f", &req).unwrap();
    assert!(!flag.enabled);
    assert_eq!(flag.description, "off");
    assert_eq!(flag.payload, json!({"v": 1}));
}

#[test]
fn flag_crud_roundtrip() {
    let conn = conn();
    upsert(&conn, "f", true, json!([]));
    assert!(get_flag(&conn, "f").unwrap().is_some());
    assert_eq!(list_flags(&conn, &[]).unwrap().len(), 1);
    assert!(delete_flag(&conn, "f").unwrap());
    assert!(get_flag(&conn, "f").unwrap().is_none());
}
