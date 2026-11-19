# AGENTS.md — blazers-dbml

DBML (Database Markup Language) parser and format generators. See the root
[`AGENTS.md`](../../AGENTS.md).

- Pure library, no I/O beyond `parse_file`: `parse`/`parse_file` →
  `Project`, then `to_sql()` (PostgreSQL DDL), `to_mermaid()` (`erDiagram`),
  `to_svg()` (standalone SVG) and `summary::summarize()` (JSON).
- Faithful port of the Go `dbml` tool — keep behaviour identical (permissive
  line scanner, same type map, same SVG layout constants).
- Consumed by: `blazers db` subcommand (`crates/blazers/src/db.rs`),
  `/api/dbml/*` routes (`blazers-server/src/dbml.rs`), `blazers_dbml_*` MCP
  tools (`blazers-mcp/src/handlers/dbml.rs`).
- `cargo test -p blazers-dbml`.
