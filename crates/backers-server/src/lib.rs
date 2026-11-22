#![recursion_limit = "512"]

//! Backers server: the HTTP API and every backend-as-a-service feature.
//!
//! Modules map to features: `auth`/`rbac` (JWT authN/authZ), `handlers`
//! (collections, records, buckets, files), `websocket`/`notification`/`log`/
//! `pubsub` (realtime), `cache`, `cronjobs`, `webhook`, `secrets`,
//! `rate_limit`, `image`, `import_export`, `openapi`, `content_type`, and
//! `app` (router assembly, embedded admin dashboard, server bootstrap).

pub mod app;
pub mod auth;
pub mod backups;
pub mod content_type;
pub mod cronjobs;
pub mod dbml;
pub mod handlers;
pub mod image;
pub mod import_export;
pub mod js;
pub mod log;
pub mod mailer;
pub mod notification;
pub mod openapi;
pub mod pubsub;
pub mod rate_limit;
pub mod rbac;
pub mod secrets;
pub mod webhook;
pub mod websocket;

// The cache store and `backers kv` protocol live in `backers-kv`; re-exported
// here so `backers_server::cache` / `backers_server::kvproto` paths keep working.
pub use backers_kv::{cache, kvproto};
