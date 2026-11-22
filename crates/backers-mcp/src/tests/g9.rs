use super::*;

/// The five `backers_flags_*` tools must appear in tools/list with all four
/// hints — reads and evaluate are read-only/idempotent; upsert is a write but
/// idempotent (same input → same flag); delete is destructive.
#[test]
fn flags_tools_listed_with_hints() {
    let (_dir, db) = test_db();
    let reply = single(request(1, "tools/list", json!({})), db);
    let tools = result_of(&reply)["tools"].as_array().expect("tools array");
    let expected = [
        ("backers_flags_delete", false, true, true),
        ("backers_flags_evaluate", true, false, true),
        ("backers_flags_get", true, false, true),
        ("backers_flags_list", true, false, true),
        ("backers_flags_upsert", false, false, true),
    ];
    for (name, read_only, destructive, idempotent) in expected {
        let tool = tools
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        let ann = &tool["annotations"];
        assert_eq!(ann["readOnlyHint"], json!(read_only), "{name}");
        assert_eq!(ann["destructiveHint"], json!(destructive), "{name}");
        assert_eq!(ann["idempotentHint"], json!(idempotent), "{name}");
        assert_eq!(ann["openWorldHint"], json!(false), "{name}");
    }
}

#[test]
fn flags_upsert_get_list_evaluate_delete_round_trip() {
    let (_dir, db) = test_db();

    let upserted = single(
        call_tool(
            2,
            "backers_flags_upsert",
            json!({
                "name": "new-ui",
                "enabled": true,
                "description": "New checkout UI",
                "rules": [{"attribute": "plan", "op": "eq", "value": "pro"}],
                "payload": {"variant": "b"}
            }),
        ),
        db.clone(),
    );
    assert!(tool_text(&upserted).contains("new-ui"));

    let fetched = single(
        call_tool(3, "backers_flags_get", json!({ "name": "new-ui" })),
        db.clone(),
    );
    assert!(tool_text(&fetched).contains("New checkout UI"));

    let listed = single(call_tool(4, "backers_flags_list", json!({})), db.clone());
    assert!(tool_text(&listed).contains("new-ui"));

    // pro matches the rule → on with payload; free does not → off.
    let evaluated = single(
        call_tool(
            5,
            "backers_flags_evaluate",
            json!({ "flags": ["new-ui"], "context": { "plan": "pro" } }),
        ),
        db.clone(),
    );
    let text = tool_text(&evaluated);
    assert!(text.contains("\"enabled\": true"), "{text}");
    assert!(text.contains("variant"), "{text}");

    let evaluated_off = single(
        call_tool(
            6,
            "backers_flags_evaluate",
            json!({ "flags": ["new-ui"], "context": { "plan": "free" } }),
        ),
        db.clone(),
    );
    assert!(tool_text(&evaluated_off).contains("\"enabled\": false"));

    let missing = single(
        call_tool(7, "backers_flags_get", json!({ "name": "nope" })),
        db.clone(),
    );
    assert!(error_text(&missing).contains("not found"));

    let deleted = single(
        call_tool(8, "backers_flags_delete", json!({ "name": "new-ui" })),
        db,
    );
    assert!(tool_text(&deleted).contains("deleted"));
}

#[test]
fn flags_upsert_rejects_an_empty_name() {
    let (_dir, db) = test_db();
    let reply = single(
        call_tool(9, "backers_flags_upsert", json!({ "name": "  " })),
        db,
    );
    assert!(error_text(&reply).contains("must not be empty"));
}
