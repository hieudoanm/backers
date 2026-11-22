//! WebSocket realtime client for `/ws`.
//!
//! Native uses `tokio-tungstenite`; `wasm32` uses `web-sys` `WebSocket`, so
//! the same API works in Node ≥22 and browsers. Lazily connects on first
//! use and reconnects with exponential backoff (1s → 15s cap).
//!
//! Protocol frames are JSON:
//! - subscribe:   `{"action":"subscribe","collection":"posts","filter":"..."}`
//! - unsubscribe: `{"action":"unsubscribe","collection":"posts"}`
//! - presence:    `{"action":"presence_join"|"presence"|"presence_leave","channel":"c","state":{...}}`
//!
//! Server → client: `{"type":"record",...}`, `{"type":"presence",...}`.

use crate::error::Result;
use crate::types::RealtimeEvent;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use native::RealtimeClient;
#[cfg(target_arch = "wasm32")]
pub use wasm::RealtimeClient;
