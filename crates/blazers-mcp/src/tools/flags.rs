//! `blazers_flags_*` tools — feature-flag management and evaluation.

use std::sync::Arc;

use super::Registered;
use super::db_tools::tool;
use crate::handlers::*;
use crate::protocol::{Annotations, PropertySchema, Schema};

const READ: Annotations = Annotations::new(true, false, true, false);

/// All five flag tools. They share the `_flags` table with the HTTP API, so a
/// flag created over MCP is immediately visible to `POST /api/flags/evaluate`.
pub(super) fn flag_tools(db: &DbHandle) -> Vec<Registered> {
    vec![
        tool(
            "blazers_flags_delete",
            "Delete a feature flag by name",
            Schema::new(
                vec![("name", PropertySchema::string("Flag name"))],
                &["name"],
            ),
            Annotations::new(false, true, true, false),
            {
                let db = Arc::clone(db);
                Box::new(move |args| handle_flags_delete(&db, args))
            },
        ),
        tool(
            "blazers_flags_evaluate",
            "Evaluate feature flags against a context object (e.g. plan, country)",
            Schema::new(
                vec![
                    (
                        "flags",
                        PropertySchema::string_list("Flag names; empty evaluates all"),
                    ),
                    (
                        "context",
                        PropertySchema::object(
                            "Attributes rules match against, e.g. {\"plan\":\"pro\"}",
                        ),
                    ),
                ],
                &[],
            ),
            READ,
            {
                let db = Arc::clone(db);
                Box::new(move |args| handle_flags_evaluate(&db, args))
            },
        ),
        tool(
            "blazers_flags_get",
            "Fetch one feature flag by name",
            Schema::new(
                vec![("name", PropertySchema::string("Flag name"))],
                &["name"],
            ),
            READ,
            {
                let db = Arc::clone(db);
                Box::new(move |args| handle_flags_get(&db, args))
            },
        ),
        tool(
            "blazers_flags_list",
            "List feature flags, optionally filtered by name",
            Schema::new(
                vec![(
                    "names",
                    PropertySchema::string_list("Optional subset of flag names"),
                )],
                &[],
            ),
            READ,
            {
                let db = Arc::clone(db);
                Box::new(move |args| handle_flags_list(&db, args))
            },
        ),
        tool(
            "blazers_flags_upsert",
            "Create or replace a feature flag (enabled, rules, payload, description)",
            Schema::new(
                vec![
                    ("name", PropertySchema::string("Flag name")),
                    (
                        "enabled",
                        PropertySchema::boolean("Whether the flag is on; default false"),
                    ),
                    (
                        "rules",
                        PropertySchema::any(
                            "Targeting rules: [{\"attribute\",\"op\",\"value\"}] — ops: eq, neq, in, contains, starts_with, ends_with, gt, gte, lt, lte",
                        ),
                    ),
                    (
                        "payload",
                        PropertySchema::any("JSON payload returned when the flag evaluates on"),
                    ),
                    (
                        "description",
                        PropertySchema::string("Human-readable description"),
                    ),
                ],
                &["name"],
            ),
            Annotations::new(false, false, true, false),
            {
                let db = Arc::clone(db);
                Box::new(move |args| handle_flags_upsert(&db, args))
            },
        ),
    ]
}
