# AGENTS.md — backers (binary)

Binary crate for Backers. See the root [`AGENTS.md`](../../AGENTS.md).

- `src/main.rs` only parses args and delegates: HTTP →
  `backers_server::app::serve_http()`, KV TCP →
  `backers_kv::kvproto::serve_kv()`, MCP → `backers_mcp::run()`
  (local DB) or `backers_mcp::run_remote()` (`--kv` proxy mode), DBML →
  `src/db.rs` (`backers db sql|mermaid|svg|all|parse`, `-` = stdin).
- `tests/api/` is the authoritative end-to-end gate — it spawns the compiled
  binary, so run `cargo test -p backers` after changing routes or startup.
- The binary name is `backers`; env vars are `BACKERS_*` (never `BACKBONE_*`).
