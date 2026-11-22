# backers-mcp

Model Context Protocol server for Backers — exposes the database as MCP tools
over stdio.

## How it works

- Newline-delimited JSON-RPC 2.0 on stdin/stdout; diagnostics on stderr.
- Opens the same SQLite database as the HTTP server (`backers_core::db`).
- `src/lib.rs` — `run()` entrypoint, `Server`, tool registration.
- `src/protocol.rs` — JSON-RPC + MCP types (no I/O).
- `src/transport.rs` — frame reader/writer.
- `src/handlers.rs` — tool implementations against a `DbHandle`
  (`Arc<Mutex<rusqlite::Connection>>`).
- `src/tools.rs` — tool schemas (`backers_*` names).
- `src/tests.rs` — protocol + handler tests (`#[cfg(test)]`).

## Tools

`backers_health`, `backers_collections_list/create/delete`,
`backers_records_list/get/create/update/delete`, `backers_export`,
`backers_import`.

Run: `backers mcp serve` (handled by the `backers` binary).

## Rules

- Handlers must do real DB work — no stubbed responses.
- `per_page` is capped at 200.
- `cargo test -p backers-mcp`.
