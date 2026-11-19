//! `blazers_dbml_*` tool handlers — pure DBML conversion, no database access.

use serde::Deserialize;

use super::*;

/// Arguments shared by every `blazers_dbml_*` tool.
#[derive(Debug, Deserialize)]
pub struct DbmlArgs {
    /// DBML source content.
    pub dbml: String,
}

fn parse_project(arguments: &str) -> Result<blazers_dbml::Project, ToolResult> {
    let args: DbmlArgs = parse_args(arguments).map_err(error_result)?;
    if args.dbml.trim().is_empty() {
        return Err(error_result("dbml argument is required"));
    }
    blazers_dbml::parse(args.dbml.as_bytes())
        .map_err(|err| error_result(format!("parse DBML: {err}")))
}

/// `blazers_dbml_sql` — DBML source to PostgreSQL DDL.
pub fn handle_dbml_sql(arguments: &str) -> ToolResult {
    match parse_project(arguments) {
        Ok(project) => text_result(project.to_sql()),
        Err(result) => result,
    }
}

/// `blazers_dbml_mermaid` — DBML source to a Mermaid `erDiagram`.
pub fn handle_dbml_mermaid(arguments: &str) -> ToolResult {
    match parse_project(arguments) {
        Ok(project) => text_result(project.to_mermaid()),
        Err(result) => result,
    }
}

/// `blazers_dbml_svg` — DBML source to a standalone SVG diagram.
pub fn handle_dbml_svg(arguments: &str) -> ToolResult {
    match parse_project(arguments) {
        Ok(project) => text_result(project.to_svg()),
        Err(result) => result,
    }
}

/// `blazers_dbml_parse` — DBML source to a JSON schema summary.
pub fn handle_dbml_parse(arguments: &str) -> ToolResult {
    match parse_project(arguments) {
        Ok(project) => match blazers_dbml::summary::summarize(&project) {
            Ok(summary) => text_result(summary),
            Err(err) => error_result(format!("summarize: {err}")),
        },
        Err(result) => result,
    }
}
