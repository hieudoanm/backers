# AGENTS.md — backers-dbml

DBML (Database Markup Language) parser and format generators. See the root
[`AGENTS.md`](../../AGENTS.md).

- Pure library, no I/O beyond `parse_file`: `parse`/`parse_file` →
  `Project`, then `to_sql()` (PostgreSQL DDL), `to_mermaid()` (`erDiagram`),
  `to_svg()` (standalone SVG) and `summary::summarize()` (JSON).
- Faithful port of the Go `dbml` tool — keep behaviour identical (permissive
  line scanner, same type map, same SVG layout constants).
- Consumed by: `backers db` subcommand (`crates/backers/src/db.rs`),
  `/api/dbml/*` routes (`backers-server/src/dbml.rs`), `backers_dbml_*` MCP
  tools (`backers-mcp/src/handlers/dbml.rs`).
- `cargo test -p backers-dbml`.
