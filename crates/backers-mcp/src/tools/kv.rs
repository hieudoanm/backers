use super::*;
use crate::protocol::{Annotations, PropertySchema, Schema, Tool};

fn kv_tool_entry(
    name: &str,
    description: &str,
    input_schema: Schema,
    annotations: Annotations,
    handler: Box<dyn Fn(&str) -> crate::protocol::ToolResult>,
) -> Registered {
    Registered {
        tool: Tool {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
            annotations,
        },
        handler,
    }
}

/// KV tools proxying a remote `backers kv` listener (`mcp serve --kv`).
/// Sorted by name like the local surface.
pub fn register_kv(addr: &str) -> Vec<Registered> {
    let addr = addr.to_string();
    let mut tools = vec![
        kv_tool_entry(
            "backers_kv_ping",
            "Ping the remote backers kv server",
            Schema::empty(),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_ping(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_set",
            "Set a key on the remote kv server, optionally with EX seconds TTL",
            Schema::new(
                vec![
                    ("key", PropertySchema::string("Key to set")),
                    ("value", PropertySchema::string("String value")),
                    ("ex", PropertySchema::integer("Optional TTL in seconds")),
                ],
                &["key", "value"],
            ),
            Annotations::new(false, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_set(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_get",
            "Get a key from the remote kv server",
            Schema::new(
                vec![("key", PropertySchema::string("Key to fetch"))],
                &["key"],
            ),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_get(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_del",
            "Delete keys from the remote kv server",
            Schema::new(
                vec![("keys", PropertySchema::string_list("Keys to delete"))],
                &["keys"],
            ),
            Annotations::new(false, true, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_del(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_exists",
            "Check whether a key exists on the remote kv server",
            Schema::new(
                vec![("key", PropertySchema::string("Key to check"))],
                &["key"],
            ),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_exists(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_keys",
            "List keys on the remote kv server, optionally filtered by a glob pattern",
            Schema::new(
                vec![(
                    "pattern",
                    PropertySchema::string("Optional glob pattern (* and ?)"),
                )],
                &[],
            ),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_keys(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_len",
            "Count keys on the remote kv server",
            Schema::empty(),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_len(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_ttl",
            "Get the TTL of a key (-1 = none, -2 = missing)",
            Schema::new(
                vec![("key", PropertySchema::string("Key to inspect"))],
                &["key"],
            ),
            Annotations::new(true, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_ttl(&addr, args))
            },
        ),
        kv_tool_entry(
            "backers_kv_expire",
            "Set a TTL in seconds on a remote kv key",
            Schema::new(
                vec![
                    ("key", PropertySchema::string("Key to expire")),
                    ("seconds", PropertySchema::integer("TTL in seconds")),
                ],
                &["key", "seconds"],
            ),
            Annotations::new(false, false, true, true),
            {
                let addr = addr.clone();
                Box::new(move |args| handle_kv_expire(&addr, args))
            },
        ),
    ];
    tools.sort_by(|a, b| a.tool.name.cmp(&b.tool.name));
    tools
}
