//! DBML (Database Markup Language) parser and format generators.
//!
//! A Rust port of the Go `dbml` tool: parse a `.dbml` file and emit
//! PostgreSQL DDL ([`Project::to_sql`]), a Mermaid `erDiagram`
//! ([`Project::to_mermaid`]) or a standalone SVG diagram
//! ([`Project::to_svg`]). Consumed by the `blazers db` subcommand, the
//! `/api/dbml/*` REST routes and the `blazers_dbml_*` MCP tools.

pub mod generate;
pub mod model;
pub mod parse;
pub mod summary;
pub mod svg;

pub use model::{Column, Enum, Index, Project, Reference, Table};
pub use parse::{parse, parse_file};

#[cfg(test)]
mod tests;
