//! Rust client SDK for the Blazers BaaS.
//!
//! Mirrors `@blazers/sdk` (TypeScript): auth (incl. OAuth2/OTP/MFA/refresh),
//! collections/records with filter/sort/expand, realtime WebSocket +
//! presence, file buckets, feature flags, edge functions and batch.
//!
//! Compiles on native targets (reqwest + tokio-tungstenite) **and**
//! `wasm32-unknown-unknown` (web-sys fetch/WebSocket — Node ≥22 or browsers):
//!
//! ```sh
//! cargo build -p blazers-sdk                          # native
//! cargo build -p blazers-sdk --target wasm32-unknown-unknown
//! ```
//!
//! ```no_run
//! use blazers_sdk::{Blazers, ClientOptions};
//!
//! # async fn demo() -> blazers_sdk::Result<()> {
//! let bz = Blazers::new(ClientOptions {
//!     url: "http://localhost:8090".into(),
//!     ..Default::default()
//! });
//! let flags = bz.flags.evaluate(&[], &Default::default()).await?;
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod error;
pub mod files;
pub mod flags;
pub mod functions;
pub mod http;
#[cfg(target_arch = "wasm32")]
pub mod js;
pub mod realtime;
pub mod records;
pub mod types;

pub use auth::{AuthClient, Credentials, MfaSetup, UpdateMeOptions};
pub use error::{BlazersError, Result};
pub use files::{FileTokenOptions, FilesClient};
pub use flags::FlagsClient;
pub use functions::FunctionsClient;
pub use http::{Body, ClientOptions, HttpClient, Multipart};
pub use realtime::RealtimeClient;
pub use records::{CollectionClient, CollectionsClient};
pub use types::*;

/// Blazers BaaS client — single entry point composing auth, records, files,
/// realtime, flags, functions and batch against a running `blazers serve`.
#[derive(Debug, Clone)]
pub struct Blazers {
    /// Low-level transport (token, headers, raw verbs).
    pub http: HttpClient,
    /// `/api/auth/*` — login stores the session token automatically.
    pub auth: AuthClient,
    /// `/api/collections*` — admin collection management.
    pub collections: CollectionsClient,
    /// `/api/buckets/*` — buckets, upload, download, file tokens.
    pub files: FilesClient,
    /// `/api/flags*` — feature flags + evaluation.
    pub flags: FlagsClient,
    /// `/api/functions*` — edge functions.
    pub functions: FunctionsClient,
    /// `/ws` — realtime subscriptions and presence.
    pub realtime: RealtimeClient,
}

impl Blazers {
    /// Builds a client from [`ClientOptions`].
    pub fn new(options: ClientOptions) -> Self {
        let http = HttpClient::new(options);
        Blazers {
            auth: AuthClient::new(http.clone()),
            collections: CollectionsClient::new(http.clone()),
            files: FilesClient::new(http.clone()),
            flags: FlagsClient::new(http.clone()),
            functions: FunctionsClient::new(http.clone()),
            realtime: RealtimeClient::new(http.clone()),
            http,
        }
    }

    /// Client for one collection's records.
    pub fn collection(&self, name: &str) -> CollectionClient {
        CollectionClient::new(self.http.clone(), name)
    }

    /// `GET /api/health` — liveness probe.
    pub async fn health(&self) -> Result<serde_json::Value> {
        self.http.get("/api/health").await
    }

    /// `POST /api/batch` — up to 50 sub-requests in one round-trip.
    pub async fn batch(&self, requests: Vec<BatchSubRequest>) -> Result<Vec<BatchResult>> {
        self.http.batch(requests).await
    }
}
