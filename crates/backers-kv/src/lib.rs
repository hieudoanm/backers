//! Backers key-value store: the shared `CacheStore` and the `backers kv`
//! inline-protocol TCP listener.
//!
//! `cache` holds the in-memory + SQLite-backed cache with typed ops (strings,
//! hashes, lists); `kvproto` is the Redis-style line protocol that shares the
//! same store. Both the HTTP cache API in `backers-server` and the standalone
//! `backers kv` command go through this crate.

pub mod cache;
pub mod kvproto;
