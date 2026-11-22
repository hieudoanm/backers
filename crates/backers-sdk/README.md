# /sdk

Rust client SDK for the [Backers](../../README.md) BaaS — auth, collections
and records, file buckets, realtime (WebSocket + presence), feature flags,
edge functions, batch requests and `GET /api/health`.

One crate, two consumers:

- **Rust** — async client for native targets _and_ `wasm32-unknown-unknown`
- **JavaScript** — the same code compiled to WebAssembly and consumed through
  this package (`@backers/sdk`) by `packages/app` and any bundler/Node ≥22

## Rust usage

```toml
[dependencies]
backers-sdk = { path = "crates/backers-sdk" }
tokio = { version = "1", features = ["full"] }
```

```rust
use backers_sdk::{Backers, ClientOptions};

#[tokio::main]
async fn main() -> backers_sdk::Result<()> {
    let bz = Backers::new(ClientOptions {
        url: "http://localhost:8090".into(),
        ..Default::default()
    });

    // Feature flags
    let flags = bz.flags.evaluate(&[], &Default::default()).await?;

    // Records
    let page = bz
        .collection("posts")
        .list(&backers_sdk::ListOptions {
            filter: Some("status=\"live\"".into()),
            ..Default::default()
        })
        .await?;

    // Realtime
    bz.realtime.connect().await?;
    bz.realtime.on_message(|ev| println!("{ev:?}"));
    bz.realtime.subscribe("posts", None, |_| {}).await?;

    Ok(())
}
```

### WASM (Rust → JS)

```sh
cargo build -p backers-sdk --target wasm32-unknown-unknown
```

The crate exports `#[wasm_bindgen]` classes (see `src/js.rs`) mirroring the
old TypeScript SDK surface. Build the JS bundle with `wasm-pack` (see
[JavaScript usage](#javascript-usage)).

## JavaScript usage

The package ships a `wasm-pack` bundle in `pkg/` — build it before consuming:

```sh
pnpm --filter @backers/sdk build        # bundler target (Next.js, Vite, …)
pnpm --filter @backers/sdk build:node   # nodejs target (plain Node ≥22)
```

Requires the `wasm32-unknown-unknown` Rust target and `wasm-pack`
(`cargo install wasm-pack` or the installer script). CI installs both.

```ts
import { Backers } from '@backers/sdk';

const bz = new Backers({ url: 'http://localhost:8090' });
bz.http.setToken(localStorage.getItem('token') ?? undefined);

// Auth — login stores the token automatically
const res = await bz.auth.login({ email, password }); // res may be an MFA challenge
await bz.auth.authWithOtp(email, otp);
await bz.auth.mfaLogin({ email, password, code });

// Records
const cols = await bz.collections.list();
const page = await bz.collection('posts').list({ filter: 'status="live"' });
await bz.collection('posts').create({ title: 'Hello' });

// Files
await bz.files.createBucket('avatars', true);
await bz.files.upload('avatars', 'a.png', new Uint8Array([…]), 'image/png');
const url = bz.files.downloadUrl('avatars', fileId);

// Feature flags
await bz.flags.upsert({ name: 'new-ui', enabled: true });
const flags = await bz.flags.evaluate(['new-ui'], { plan: 'pro' });

// Realtime — onMessage gets every frame; returns an unsubscribe fn
const off = bz.realtime.onMessage((event) => console.log(event));
await bz.realtime.connect();
await bz.realtime.subscribe('posts', { filter: 'status="live"' });
await bz.realtime.presenceJoin('demo:room', { state: { demo: true } });
off();
```

Async methods return `Promise`s; arguments and results are plain JS objects
(`serde-wasm-bindgen`). Errors reject with the `BackersError` message.

### API surface

| JS access                    | Rust equivalent                                                                  |
| ---------------------------- | -------------------------------------------------------------------------------- |
| `bz.auth.*`                  | `AuthClient` — register, login, MFA, OTP, refresh, me, password reset, OAuth URL |
| `bz.collections.*`           | `CollectionsClient` — admin CRUD + rules                                         |
| `bz.collection(name).*`      | `CollectionClient` — list/get/create/update/delete                               |
| `bz.files.*`                 | `FilesClient` — buckets, upload (`Uint8Array`), download, file tokens            |
| `bz.flags.*`                 | `FlagsClient` — CRUD + `evaluate(names, context)`                                |
| `bz.functions.*`             | `FunctionsClient` — CRUD + `invoke(name, input)`                                 |
| `bz.realtime.*`              | `RealtimeClient` — subscribe, presence, `onMessage`                              |
| `bz.http.setToken/authToken` | `HttpClient` token management                                                    |
| `bz.health()`                | `GET /api/health`                                                                |

Naming follows the previous TypeScript SDK (`perPage`, `listRule`,
`authWithOtp`, … are accepted as camelCase aliases).

## Layout

| Path               | What it is                                                         |
| ------------------ | ------------------------------------------------------------------ |
| `src/lib.rs`       | `Backers` facade + re-exports                                      |
| `src/http.rs`      | transport (`reqwest` native / `web-sys` wasm), `Body`, `Multipart` |
| `src/auth.rs`      | `/api/auth/*`                                                      |
| `src/records.rs`   | `CollectionClient` + `CollectionsClient`                           |
| `src/files.rs`     | `/api/buckets/*`                                                   |
| `src/flags.rs`     | `/api/flags/*`                                                     |
| `src/functions.rs` | `/api/functions/*`                                                 |
| `src/realtime.rs`  | `/ws` — `native` (tokio-tungstenite) / `wasm` (web-sys)            |
| `src/types.rs`     | wire types                                                         |
| `src/error.rs`     | `BackersError`                                                     |
| `src/js.rs`        | `#[wasm_bindgen]` facade (wasm32 only)                             |
| `package.json`     | `@backers/sdk` — wasm-pack bundle → `pkg/`                         |

## Commands

```sh
cargo check -p backers-sdk                                    # native
cargo check -p backers-sdk --target wasm32-unknown-unknown    # wasm
cargo test -p backers-sdk                                     # unit + mock-server tests
pnpm --filter @backers/sdk build                              # JS bundle (needs wasm-pack)
```
