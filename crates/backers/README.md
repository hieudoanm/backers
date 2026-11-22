# backers

The Backers binary — a thin argument-parsing entrypoint over the library
crates.

- `src/main.rs` — `backers` / `backers serve` → `backers_server::app::serve_http()`;
  `backers mcp serve` → `backers_mcp::run(stderr)`; `--help` / `--version`.
- `tests/api.rs` — end-to-end suite: builds the real binary
  (`CARGO_BIN_EXE_backers`), boots it on a random port with a temp
  `BACKERS_DATA` dir, and exercises health/auth/CRUD over HTTP.

Keep `main.rs` thin — all logic lives in `backers-server` / `backers-mcp`.
