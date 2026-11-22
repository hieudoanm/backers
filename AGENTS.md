# AGENTS.md — Backers

Guidance for AI agents working in this repository.

## What this is

Backers is a single-binary Back-end as a Service written in Rust — auth, a
schema-less document database on SQLite, file buckets, realtime (WebSocket +
SSE), webhooks, cron jobs, an encrypted secrets vault, cache, pub/sub,
notifications, an embedded admin dashboard, OpenAPI docs and an MCP server.
Inspired by PocketBase and Supabase; see the feature comparison in
[`README.md`](./README.md).

## Repository layout

| Path                        | What it is                                                                                                                                                               |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/backers-core`       | Storage layer: `db/` (schema, migrations, all SQL, split by domain) + `models/` (wire types, `AppError`, split by domain). See `crates/backers-core/AGENTS.md`.          |
| `crates/backers-kv`         | Key-value store: shared `CacheStore` (typed ops) + `kvproto` (Redis-style TCP for `backers kv`). See `crates/backers-kv/AGENTS.md`.                                      |
| `crates/backers-dbml`       | DBML parser + SQL/Mermaid/SVG generators — powers `backers db`, `/api/dbml/*`, `backers_dbml_*` MCP tools. See `crates/backers-dbml/AGENTS.md`.                          |
| `crates/backers-server`     | Everything HTTP: feature modules + `app.rs` (router, embedded dashboard via `rust-embed`, `serve_http`). See `crates/backers-server/AGENTS.md`.                          |
| `crates/backers-mcp`        | MCP stdio server (newline-delimited JSON-RPC 2.0) exposing the DB as tools. See `crates/backers-mcp/AGENTS.md`.                                                          |
| `crates/backers-sdk`        | `@backers/sdk` — Rust client SDK (native + `wasm32-unknown-unknown`) with a `#[wasm_bindgen]` JS facade; `wasm-pack build` → `pkg/`. See `crates/backers-sdk/AGENTS.md`. |
| `crates/backers`            | Thin binary: `backers` / `backers serve` → HTTP, `backers mcp serve` → MCP.                                                                                              |
| `packages/landing`          | Next.js + Tailwind landing page **and** docs site (`/docs/*`) → GitHub Pages. See `packages/landing/AGENTS.md`.                                                          |
| `packages/app`              | `@backers/app` — live demo SPA (Next.js + Tailwind + DaisyUI) exercising the SDK via `workspace:*`. `npm run dev` → :3001. See `packages/app/AGENTS.md`.                 |
| `.github/workflows/ci.yaml` | Pages deploy + release binary jobs.                                                                                                                                      |

## Commands

```bash
cargo build --workspace          # debug build
cargo test --workspace           # all tests (unit + integration)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
make all                         # fmt-check + clippy + test + release build
```

JS workspace (`packages/{landing,sdk,app}` — pnpm + Turborepo):

```bash
pnpm install        # workspace install (never npm install — pnpm manages it)
pnpm dev            # turbo run dev → landing :3000 + app :3001
pnpm build          # turbo run build (sdk dist + both Next apps)
pnpm lint           # turbo run lint
pnpm test           # turbo run test (vitest)
pnpm typecheck      # turbo run typecheck
pnpm format         # turbo run format (prettier --write)
```

Run locally: `cargo run -p backers -- serve` (or `./target/release/backers`).
Data lives in `$BACKERS_DATA` (default `~/.backers`): `data.db`, `secrets.key`,
`uploads/`.

## Conventions

- **Rust 2024 edition**, `rust-version = 1.91`-era toolchain; let-chains are used.
- Dependency versions are pinned with `=` in crate manifests — match that style
  when adding deps; prefer versions published >7 days ago.
- Errors are explicit: libraries return typed errors (`models::AppError` maps to
  HTTP statuses); no silent `unwrap()` on I/O or external input.
- `backers-core` must not depend on `backers-server`/`backers-kv`;
  `backers-server` owns all axum handlers; `backers-mcp` depends on
  `backers-core` only. Keep the dependency graph acyclic.
- New HTTP features: add the module under `crates/backers-server/src/`, declare
  it `pub mod` in `lib.rs`, register routes in `app.rs`.
- Unit tests live in per-module `tests/` directories next to the code
  (`src/db/tests/`, `src/app/tests/`, …); the `crates/backers/tests/api/`
  suite builds the real binary end-to-end. Keep files ≲200 lines.
- Env vars: `PORT`, `BACKERS_DATA`, `JWT_SECRET`, `BACKERS_SECRETS_KEY`.
  Rebrand-safe: never reintroduce `BACKBONE_*` names.
- Do not commit secrets, `data.db`, or `target/`.
