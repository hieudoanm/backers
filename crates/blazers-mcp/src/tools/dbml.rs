//! `blazers_dbml_*` tools — stateless DBML conversion, no database handle.

use super::Registered;
use super::db_tools::tool;
use crate::handlers::dbml::*;
use crate::protocol::{Annotations, PropertySchema, Schema};

fn dbml_schema() -> Schema {
    Schema::new(
        vec![("dbml", PropertySchema::string("DBML source content"))],
        &["dbml"],
    )
}

/// All four DBML tools. Conversion is a pure function of its input: read-only,
/// idempotent, and entirely local (`openWorldHint` stays false).
pub(super) fn dbml_tools() -> Vec<Registered> {
    vec![
        tool(
            "blazers_dbml_mermaid",
            "Convert DBML source to a Mermaid ER diagram",
            dbml_schema(),
            Annotations::new(true, false, true, false),
            Box::new(handle_dbml_mermaid),
        ),
        tool(
            "blazers_dbml_parse",
            "Parse DBML source and return a JSON summary of the schema",
            dbml_schema(),
            Annotations::new(true, false, true, false),
            Box::new(handle_dbml_parse),
        ),
        tool(
            "blazers_dbml_sql",
            "Convert DBML source to a PostgreSQL SQL schema",
            dbml_schema(),
            Annotations::new(true, false, true, false),
            Box::new(handle_dbml_sql),
        ),
        tool(
            "blazers_dbml_svg",
            "Render DBML source as an SVG ER diagram",
            dbml_schema(),
            Annotations::new(true, false, true, false),
            Box::new(handle_dbml_svg),
        ),
    ]
}
