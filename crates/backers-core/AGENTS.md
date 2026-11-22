# AGENTS.md — backers-core

Storage crate for Backers. See the root [`AGENTS.md`](../../AGENTS.md).

- `src/db/`: schema + migrations (`migrate.rs`, `core.rs`) + all SQL helpers,
  one file per domain (`collections.rs`, `records.rs` + `records_query.rs`
  for filter/sort listing, `cache.rs`, `settings.rs`, …). `data_dir()`
  resolves `$BACKERS_DATA` else `~/.backers`.
- `src/filter/`: PocketBase-style filter expression engine (tokenizer,
  parser, evaluator, sort keys) shared by `?filter=` and collection record
  rules.
- `src/models/`: wire types + `AppError` (`IntoResponse` → JSON + status),
  one file per domain.
- No HTTP handlers, no feature logic — features live in `backers-server`.
- Must stay dependency-light: only `backers-*`-free crates (axum is used solely
  for `AppError`'s `IntoResponse`).
- Tests live in `src/db/tests/` and `src/models/tests/`; run
  `cargo test -p backers-core`.
