use super::*;
use crate::protocol;

/// Every registered tool must declare all four MCP hints as booleans —
/// directories like OpenAI's reject listings where any hint is missing.
#[test]
fn every_tool_declares_all_four_hints() {
    let (_dir, db) = test_db();
    let reply = single(request(2, "tools/list", json!({})), db);
    let tools = result_of(&reply)["tools"].as_array().expect("tools array");
    assert!(!tools.is_empty());
    for tool in tools {
        let name = tool["name"].as_str().expect("tool name");
        let annotations = &tool["annotations"];
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(
                annotations.get(hint).is_some_and(|v| v.is_boolean()),
                "{name} missing boolean annotation {hint}: {annotations}"
            );
        }
    }
}

/// Reads tools/list from a remote-mode server with the KV surface registered.
fn kv_tools() -> Vec<Value> {
    let mut output: Vec<u8> = Vec::new();
    {
        let mut server = Server::remote(&mut output, Diagnostics::Sink);
        server.register_kv("127.0.0.1:1");
        let input = format!(
            "{}\n",
            json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })
        );
        server.serve(input.as_bytes()).expect("serve");
    }
    let replies = frames(&output);
    assert_eq!(replies.len(), 1);
    result_of(&replies[0])["tools"]
        .as_array()
        .expect("tools array")
        .clone()
}

#[test]
fn kv_surface_lists_all_nine_tools_with_hints() {
    let names: Vec<String> = kv_tools()
        .iter()
        .map(|t| t["name"].as_str().expect("tool name").to_string())
        .collect();
    for expected in [
        "backers_kv_ping",
        "backers_kv_set",
        "backers_kv_get",
        "backers_kv_del",
        "backers_kv_exists",
        "backers_kv_keys",
        "backers_kv_len",
        "backers_kv_ttl",
        "backers_kv_expire",
    ] {
        assert!(names.iter().any(|n| n == expected), "missing {expected}");
    }
    for tool in kv_tools() {
        let annotations = &tool["annotations"];
        assert_eq!(annotations["openWorldHint"], json!(true));
        for hint in ["readOnlyHint", "destructiveHint", "idempotentHint"] {
            assert!(annotations[hint].is_boolean(), "bad hint on tool");
        }
    }
}

#[test]
fn kv_tools_dispatch_and_report_connection_errors() {
    let mut output: Vec<u8> = Vec::new();
    {
        let mut server = Server::remote(&mut output, Diagnostics::Sink);
        server.register_kv("127.0.0.1:1");
        let input = format!(
            "{}\n",
            json!({
                "jsonrpc": "2.0", "id": 9, "method": "tools/call",
                "params": { "name": "backers_kv_ping", "arguments": {} }
            })
        );
        server.serve(input.as_bytes()).expect("serve");
    }
    let replies = frames(&output);
    assert_eq!(replies.len(), 1);
    // Nothing listens on port 1 — the tool must surface an error result,
    // not crash the server.
    assert_eq!(result_of(&replies[0])["isError"], json!(true));
}

#[test]
fn annotations_serialize_with_camel_case_keys() {
    let ann = protocol::Annotations::new(true, false, true, false);
    let value = serde_json::to_value(ann).expect("serialize");
    assert_eq!(
        value,
        json!({
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false,
        })
    );
}
