# Contributing to backers

Thanks for contributing to Backers, a single-binary Back-end as a Service in
Rust — auth, a schema-less document database on SQLite, file storage,
realtime, webhooks, cron jobs, secrets, pub/sub, an admin dashboard, OpenAPI,
and an MCP server.

## Repository layout

- `crates/backers-core` — storage layer (`db/` + `models/`).
- `crates/backers-server` — everything HTTP (feature modules + `app.rs`).
- `crates/backers-mcp` — MCP stdio server exposing the DB as tools.
- `crates/backers` — thin binary (`backers serve`, `backers mcp serve`).
- `packages/landing` — Next.js landing page and docs site.

See the crate-level AGENTS.md files for per-crate conventions.

## Setup

```bash
git clone https://github.com/hieudoanm/backers.git
cd backers
cargo build --workspace
cargo run -p backers -- serve
```

Data lives in `$BACKERS_DATA` (default `~/.backers`).

## Checks

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
make all   # fmt-check + clippy + test + release build
```

## Conventions

- Rust 2024 edition; dependency versions are pinned with `=` in crate
  manifests — match that style.
- `backers-core` must not depend on `backers-server`; keep the dependency
  graph acyclic.
- Errors are explicit typed errors (`models::AppError`) — no silent
  `unwrap()` on I/O or external input.
- Rebrand-safe: never reintroduce `BACKBONE_*` env var names.
- Do not commit secrets, `data.db`, or `target/`.

## License

By contributing, you agree that your contributions will be licensed under the
repository license in [`LICENSE`](LICENSE).
