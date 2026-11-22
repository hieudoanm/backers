//! OpenAPI paths for feature flags (`/api/flags`).

pub fn paths() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "/api/flags": {
            "get": {
                "summary": "List feature flags (admin)",
                "description": "Returns every flag, or a `?names=a,b` subset. Each flag has `enabled`, JSON `rules` (all must match), `payload` and `description`.",
                "tags": ["Flags"],
                "parameters": [{"name": "names", "in": "query", "required": false, "schema": {"type": "string"}, "description": "Comma-separated flag names"}],
                "responses": {"200": {"description": "Array of flags"}, "403": {"description": "Admin role required"}}
            },
            "post": {
                "summary": "Create or replace a feature flag (admin)",
                "tags": ["Flags"],
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object", "properties": {
                    "name": {"type": "string"},
                    "enabled": {"type": "boolean", "default": false},
                    "rules": {"type": "array", "items": {"type": "object", "properties": {
                        "attribute": {"type": "string", "description": "Context attribute, e.g. email, user_id, plan"},
                        "op": {"type": "string", "enum": ["eq", "neq", "in", "contains", "starts_with", "ends_with", "gt", "gte", "lt", "lte"]},
                        "value": {}
                    }, "required": ["attribute", "op", "value"]}},
                    "payload": {"description": "Arbitrary JSON returned when the flag evaluates on"},
                    "description": {"type": "string"}
                }, "required": ["name"]}}}},
                "responses": {"200": {"description": "Stored flag"}, "400": {"description": "name required"}, "403": {"description": "Admin role required"}}
            }
        },
        "/api/flags/evaluate": {
            "post": {
                "summary": "Evaluate feature flags for the caller",
                "description": "Resolves flags against auth claims (`user_id`, `email` when a Bearer token is sent) plus request `context` attributes. A flag is on when `enabled` and every rule matches. `flags` empty evaluates all flags.",
                "tags": ["Flags"],
                "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object", "properties": {
                    "flags": {"type": "array", "items": {"type": "string"}},
                    "context": {"type": "object", "description": "Extra targeting attributes"}
                }}}}},
                "responses": {"200": {"description": "Map of flag name to {enabled, payload?}"}}
            }
        },
        "/api/flags/{name}": {
            "get": {
                "summary": "Get one feature flag (admin)",
                "tags": ["Flags"],
                "parameters": [{"name": "name", "in": "path", "required": true, "schema": {"type": "string"}}],
                "responses": {"200": {"description": "The flag"}, "403": {"description": "Admin role required"}, "404": {"description": "Flag not found"}}
            },
            "patch": {
                "summary": "Partially update a feature flag (admin)",
                "tags": ["Flags"],
                "parameters": [{"name": "name", "in": "path", "required": true, "schema": {"type": "string"}}],
                "requestBody": {"content": {"application/json": {"schema": {"type": "object", "properties": {
                    "enabled": {"type": "boolean"},
                    "rules": {"type": "array"},
                    "payload": {},
                    "description": {"type": "string"}
                }}}}},
                "responses": {"200": {"description": "Updated flag"}, "403": {"description": "Admin role required"}, "404": {"description": "Flag not found"}}
            },
            "delete": {
                "summary": "Delete a feature flag (admin)",
                "tags": ["Flags"],
                "parameters": [{"name": "name", "in": "path", "required": true, "schema": {"type": "string"}}],
                "responses": {"204": {"description": "Deleted"}, "403": {"description": "Admin role required"}, "404": {"description": "Flag not found"}}
            }
        }
    })
    .as_object()
    .unwrap()
    .clone()
}
