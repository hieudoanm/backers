# AGENTS.md — backers-mcp

MCP stdio server crate. See the root [`AGENTS.md`](../../AGENTS.md).

- Entry point: `backers_mcp::run(&mut dyn Write)` — called by the `backers`
  binary for `backers mcp serve`. `run_remote(addr, ...)` serves `backers_kv_*`
  tools proxying a remote `backers kv` listener (`--kv HOST:PORT`).
- Framing: one JSON-RPC 2.0 message per line on stdin/stdout; nothing but
  frames on stdout.
- Depends on `backers-core` only (db/models, including `ExportData` and
  `db::export_all`/`db::import_all`). Do not add HTTP handler deps here.
- Tool names are `backers_*`; keep them stable — MCP clients cache schemas.
- Layout: `handlers/` (db/io/kv tool handlers), `tools/` (tool
  registration), `protocol/` (wire types + JSON schema), `server.rs` +
  `server/dispatch.rs` (request dispatch). Tests in `src/tests/`.
- `cargo test -p backers-mcp`.
