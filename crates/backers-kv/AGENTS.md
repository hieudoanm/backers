# AGENTS.md — backers-kv

Key-value crate for Backers. See the root [`AGENTS.md`](../../AGENTS.md).

- `src/cache/` — `CacheStore`: in-memory + SQLite-backed cache with typed ops
  (strings, hashes, lists) shared by the REST cache API in `backers-server`.
- `src/kvproto/` — Redis-style inline-protocol TCP listener (`backers kv`
  command), sharing the same `CacheStore`.
- Depends on `backers-core` (db/models) only; no HTTP handlers — the REST
  surface lives in `backers-server/src/handlers/cache*.rs`.
- Tests live in per-module `tests/` dirs; `cargo test -p backers-kv`.
