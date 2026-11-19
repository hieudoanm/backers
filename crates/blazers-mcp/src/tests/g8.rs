use super::*;

const DBML: &str = "Table users {\n  id integer [pk]\n  email varchar(255) [unique, not_null]\n}\n";

/// The four `blazers_dbml_*` tools must appear in tools/list with all four
/// hints — they are pure local conversions, so readOnly+idempotent and never
/// open-world.
#[test]
fn dbml_tools_listed_with_hints() {
    let (_dir, db) = test_db();
    let reply = single(request(1, "tools/list", json!({})), db);
    let tools = result_of(&reply)["tools"].as_array().expect("tools array");
    for name in [
        "blazers_dbml_mermaid",
        "blazers_dbml_parse",
        "blazers_dbml_sql",
        "blazers_dbml_svg",
    ] {
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        let ann = &tool["annotations"];
        assert_eq!(ann["readOnlyHint"], json!(true), "{name}");
        assert_eq!(ann["destructiveHint"], json!(false), "{name}");
        assert_eq!(ann["idempotentHint"], json!(true), "{name}");
        assert_eq!(ann["openWorldHint"], json!(false), "{name}");
    }
}

#[test]
fn dbml_sql_tool_converts_source() {
    let (_dir, db) = test_db();
    let reply = single(call_tool(2, "blazers_dbml_sql", json!({"dbml": DBML})), db);
    let text = tool_text(&reply);
    assert!(text.contains("CREATE TABLE users"));
    assert!(text.contains("email VARCHAR(255) UNIQUE NOT NULL"));
}

#[test]
fn dbml_parse_tool_returns_json_summary() {
    let (_dir, db) = test_db();
    let reply = single(
        call_tool(3, "blazers_dbml_parse", json!({"dbml": DBML})),
        db,
    );
    let text = tool_text(&reply);
    let summary: Value = serde_json::from_str(&text).expect("summary json");
    assert_eq!(summary["tables"][0]["name"], "users");
}

#[test]
fn dbml_tools_reject_missing_source() {
    let (_dir, db) = test_db();
    let reply = single(call_tool(4, "blazers_dbml_sql", json!({})), db);
    assert!(error_text(&reply).contains("dbml"));
}
