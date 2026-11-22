# AGENTS.md — blazers-sdk

Rust client SDK for the Blazers BaaS — published to JS as `@blazers/sdk`
via the `#[wasm_bindgen]` facade in `src/js.rs` + `package.json`
(`wasm-pack build` → `pkg/`). It replaced the former TypeScript SDK in
`packages/sdk` (deleted); keep the JS facade API-compatible (`bz.auth.login`,
`bz.collection(name)`, `bz.realtime.onMessage`, …) when the HTTP API or
realtime protocol changes.

## What this is

Async client covering the whole `/api/*` surface: auth (register, login,
MFA/TOTP, refresh, OTP, OAuth URL, anonymous), collections + records
(filter/sort/expand/search), file buckets (multipart upload, download URLs,
file tokens), feature flags, edge functions, batch, and the `/ws` realtime
protocol (record events + presence).

## Dual-target

The crate compiles on **native** and **`wasm32-unknown-unknown`**:

- Native (`cfg(not(target_arch = "wasm32"))`): `reqwest` for HTTP,
  `tokio-tungstenite` for `/ws`. Needs a Tokio runtime.
- WASM: `web-sys` `fetch`/`WebSocket` — runs on Node ≥22 and browsers.
  Transport lives in `src/http/transport_{native,wasm}.rs`; realtime in
  `src/realtime/{native,wasm}.rs`.

Verify both:

```bash
cargo check -p blazers-sdk
cargo check -p blazers-sdk --target wasm32-unknown-unknown
cargo test -p blazers-sdk   # native only (tests are cfg'd out on wasm)
```

JS consumers use the generated `pkg/` bundle (`wasm-pack --target bundler`
for Next.js/Vite, `--target nodejs` for plain Node ≥22). The facade lives in
`src/js.rs` and is `#[cfg(target_arch = "wasm32")]`-gated; verify it with
`wasm-bindgen --target nodejs` smoke tests or `pnpm --filter @blazers/app
build`.

## Layout

| Path                   | What it is                                                                                                                                   |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/lib.rs`           | `Blazers` facade composing all sub-clients + type re-exports                                                                                 |
| `src/http.rs`          | `HttpClient`, `ClientOptions`, `Body`/`Multipart`, verbs, `batch()`                                                                          |
| `src/http/transport_*` | per-target `send()` (reqwest / web-sys fetch)                                                                                                |
| `src/error.rs`         | `BlazersError` (Http status+body / Transport / Codec)                                                                                        |
| `src/types.rs`         | wire types mirroring `blazers-core/src/models` + realtime event enums                                                                        |
| `src/auth.rs`          | `AuthClient` — `/api/auth/*`, stores session token on login                                                                                  |
| `src/records.rs`       | `CollectionClient` (per-collection CRUD) + `CollectionsClient` (admin)                                                                       |
| `src/files.rs`         | `FilesClient` — buckets, upload/download, file tokens                                                                                        |
| `src/flags.rs`         | `FlagsClient` — `/api/flags*` incl. `evaluate`                                                                                               |
| `src/functions.rs`     | `FunctionsClient` — edge-function CRUD + `invoke`                                                                                            |
| `src/realtime.rs`      | `RealtimeClient` — subscribe/presence, auto-reconnect                                                                                        |
| `src/realtime/*`       | per-target socket impls                                                                                                                      |
| `src/js.rs`            | `#[wasm_bindgen]` JS facade — `Blazers`, `AuthJs`, `CollectionJs`, `FilesJs`, `FlagsJs`, `FunctionsJs`, `RealtimeJs`, `HttpJs` (wasm32 only) |
| `tests/client.rs`      | integration tests vs a canned TCP HTTP server (native only)                                                                                  |

## Conventions

- Field names match the wire format exactly (snake_case); request-side
  camelCase conveniences from TS (`perPage`, `currentPassword`) map to
  snake_case struct fields here — this is idiomatic Rust, not a 1:1 port.
- Token storage is `Arc<RwLock<Option<String>>>` inside `HttpClient` —
  `set_token` is sync and safe to call from `&self` methods.
- `url_encode` in `records.rs` is the shared `encodeURIComponent` port —
  reuse it for path segments and query values.
- Errors are explicit `Result<T, BlazersError>`; no panics on I/O.
- `cargo clippy -p blazers-sdk --all-targets -- -D warnings` and `cargo fmt`
  must stay clean on **both** targets.
