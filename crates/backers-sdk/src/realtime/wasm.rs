//! WASM realtime — `web-sys` `WebSocket` (Node ≥22 / browsers).
//!
//! Frames sent before the socket opens are queued and flushed on `open`.
//! Reconnect uses `setTimeout` backoff (1s → 15s cap), same as native.

use std::any::Any;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use serde_json::{Value, json};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, WebSocket};

use super::*;
use crate::http::HttpClient;

type Handler = Arc<dyn Fn(RealtimeEvent)>;

struct Inner {
    ws: Option<WebSocket>,
    /// Keeps event-handler closures alive for the socket's lifetime.
    closures: Vec<Box<dyn Any>>,
    url: Option<String>,
    /// Frames queued while the socket is CONNECTING.
    pending: Vec<String>,
}

struct Shared {
    inner: Mutex<Inner>,
    subs: Mutex<HashMap<String, Handler>>,
    presences: Mutex<HashMap<String, Handler>>,
    handlers: Mutex<Vec<(u64, Handler)>>,
    next_handler: AtomicU64,
    sub_filters: Mutex<HashMap<String, Option<String>>>,
    presence_states: Mutex<HashMap<String, Option<Value>>>,
    connected: AtomicBool,
    closed: AtomicBool,
    backoff_step: Mutex<u32>,
}

/// WebSocket realtime client. Cheap to clone; shares one connection.
#[derive(Clone)]
pub struct RealtimeClient {
    http: HttpClient,
    shared: Arc<Shared>,
}

fn lock<'a, T>(m: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl RealtimeClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        RealtimeClient {
            http,
            shared: Arc::new(Shared {
                inner: Mutex::new(Inner {
                    ws: None,
                    closures: Vec::new(),
                    url: None,
                    pending: Vec::new(),
                }),
                subs: Mutex::new(HashMap::new()),
                presences: Mutex::new(HashMap::new()),
                handlers: Mutex::new(Vec::new()),
                next_handler: AtomicU64::new(1),
                sub_filters: Mutex::new(HashMap::new()),
                presence_states: Mutex::new(HashMap::new()),
                connected: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                backoff_step: Mutex::new(0),
            }),
        }
    }

    /// Whether the socket is currently open.
    pub fn connected(&self) -> bool {
        self.shared.connected.load(Ordering::SeqCst)
    }

    /// Ensures a socket exists (creating one if closed). Frames queue until
    /// `open`, so callers don't need to await connectivity.
    pub async fn connect(&self) -> Result<()> {
        self.shared.closed.store(false, Ordering::SeqCst);
        let url = self.http.ws_url();
        ensure_socket(&self.shared, url);
        Ok(())
    }

    /// Subscribe to record events for `collection` (`"*"` = all collections).
    pub async fn subscribe(
        &self,
        collection: &str,
        filter: Option<&str>,
        on_event: impl Fn(RealtimeEvent) + 'static,
    ) -> Result<()> {
        lock(&self.shared.subs).insert(collection.to_string(), Arc::new(on_event));
        lock(&self.shared.sub_filters).insert(collection.to_string(), filter.map(str::to_string));
        self.connect().await?;
        let mut frame = json!({ "action": "subscribe", "collection": collection });
        if let Some(f) = filter {
            frame["filter"] = json!(f);
        }
        send_or_queue(&self.shared, frame.to_string());
        Ok(())
    }

    /// Removes a subscription.
    pub async fn unsubscribe(&self, collection: &str) -> Result<()> {
        lock(&self.shared.subs).remove(collection);
        lock(&self.shared.sub_filters).remove(collection);
        send_or_queue(
            &self.shared,
            json!({ "action": "unsubscribe", "collection": collection }).to_string(),
        );
        Ok(())
    }

    /// Join a presence channel (or update state with [`presence_update`](Self::presence_update)).
    pub async fn presence_join(
        &self,
        channel: &str,
        state: Option<Value>,
        on_event: impl Fn(RealtimeEvent) + 'static,
    ) -> Result<()> {
        lock(&self.shared.presences).insert(channel.to_string(), Arc::new(on_event));
        lock(&self.shared.presence_states).insert(channel.to_string(), state.clone());
        self.connect().await?;
        let mut frame = json!({ "action": "presence_join", "channel": channel });
        if let Some(s) = state {
            frame["state"] = s;
        }
        send_or_queue(&self.shared, frame.to_string());
        Ok(())
    }

    /// Update presence state on a joined channel.
    pub async fn presence_update(&self, channel: &str, state: Value) -> Result<()> {
        send_or_queue(
            &self.shared,
            json!({ "action": "presence", "channel": channel, "state": state }).to_string(),
        );
        Ok(())
    }

    /// Leave a presence channel.
    pub async fn presence_leave(&self, channel: &str) -> Result<()> {
        lock(&self.shared.presences).remove(channel);
        lock(&self.shared.presence_states).remove(channel);
        send_or_queue(
            &self.shared,
            json!({ "action": "presence_leave", "channel": channel }).to_string(),
        );
        Ok(())
    }

    /// Raw frame listener for frames not covered by subscribe/presence.
    /// Returns a handle usable with [`off_message`](Self::off_message).
    pub fn on_message(&self, handler: impl Fn(RealtimeEvent) + 'static) -> u64 {
        let id = self.shared.next_handler.fetch_add(1, Ordering::SeqCst);
        lock(&self.shared.handlers).push((id, Arc::new(handler)));
        id
    }

    /// Removes a handler registered via [`on_message`](Self::on_message).
    pub fn off_message(&self, id: u64) {
        lock(&self.shared.handlers).retain(|(hid, _)| *hid != id);
    }

    /// Permanently closes the socket (no reconnect).
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        self.shared.connected.store(false, Ordering::SeqCst);
        let mut inner = lock(&self.shared.inner);
        inner.pending.clear();
        if let Some(ws) = inner.ws.take() {
            ws.close().ok();
        }
        inner.closures.clear();
    }
}

/// Creates and wires a new socket unless one is already connecting/open.
fn ensure_socket(shared: &Arc<Shared>, url: String) {
    {
        let mut inner = lock(&shared.inner);
        if let Some(ws) = &inner.ws {
            let state = ws.ready_state();
            // CONNECTING(0) or OPEN(1) — keep it.
            if state == WebSocket::CONNECTING || state == WebSocket::OPEN {
                return;
            }
        }
        inner.closures.clear();
        inner.url = Some(url.clone());
    }

    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(_) => {
            schedule_reconnect(Arc::downgrade(shared), 0);
            return;
        }
    };

    let weak = Arc::downgrade(shared);
    let on_open = Closure::wrap(Box::new(move |_ev: Event| {
        let Some(shared) = weak.upgrade() else { return };
        on_open(&shared);
    }) as Box<dyn FnMut(Event)>);
    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

    let weak = Arc::downgrade(shared);
    let on_message = Closure::wrap(Box::new(move |ev: MessageEvent| {
        let Some(shared) = weak.upgrade() else { return };
        on_message(&shared, &ev);
    }) as Box<dyn FnMut(MessageEvent)>);
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

    let weak = Arc::downgrade(shared);
    let on_close = Closure::wrap(Box::new(move |_ev: CloseEvent| {
        let Some(shared) = weak.upgrade() else { return };
        on_close(&shared);
    }) as Box<dyn FnMut(CloseEvent)>);
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

    let weak = Arc::downgrade(shared);
    let on_error = Closure::wrap(Box::new(move |_ev: ErrorEvent| {
        let Some(shared) = weak.upgrade() else { return };
        on_error(&shared);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

    {
        let mut inner = lock(&shared.inner);
        inner.ws = Some(ws);
        inner.closures.push(Box::new(on_open) as Box<dyn Any>);
        inner.closures.push(Box::new(on_message) as Box<dyn Any>);
        inner.closures.push(Box::new(on_close) as Box<dyn Any>);
        inner.closures.push(Box::new(on_error) as Box<dyn Any>);
    }
}

fn on_open(shared: &Arc<Shared>) {
    shared.connected.store(true, Ordering::SeqCst);
    if let Ok(mut step) = shared.backoff_step.lock() {
        *step = 0;
    }
    let pending: Vec<String> = {
        let mut inner = lock(&shared.inner);
        std::mem::take(&mut inner.pending)
    };
    for text in pending {
        send_text(shared, &text);
    }
    resubscribe_all(shared);
}

fn on_message(shared: &Arc<Shared>, ev: &MessageEvent) {
    let Some(text) = ev.data().as_string() else {
        return;
    };
    dispatch(shared, &text);
}

fn on_close(shared: &Arc<Shared>) {
    shared.connected.store(false, Ordering::SeqCst);
    schedule_reconnect(Arc::downgrade(shared), next_step(shared));
}

fn on_error(shared: &Arc<Shared>) {
    shared.connected.store(false, Ordering::SeqCst);
    schedule_reconnect(Arc::downgrade(shared), next_step(shared));
}

fn next_step(shared: &Arc<Shared>) -> u32 {
    let mut step = lock(&shared.backoff_step);
    *step += 1;
    *step
}

fn schedule_reconnect(weak: Weak<Shared>, step: u32) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let delay = (1000u32 * 2u32.saturating_pow(step.min(4))).min(15000);
    let cb = Closure::wrap(Box::new(move || {
        let Some(shared) = weak.upgrade() else { return };
        if shared.closed.load(Ordering::SeqCst) {
            return;
        }
        let url = lock(&shared.inner).url.clone();
        if let Some(url) = url {
            ensure_socket(&shared, url);
        }
    }) as Box<dyn FnMut()>);
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            delay as i32,
        )
        .ok();
    // The timeout closure is one-shot; leaking it is fine (tiny, bounded).
    cb.forget();
}

fn send_or_queue(shared: &Arc<Shared>, text: String) {
    if shared.connected.load(Ordering::SeqCst) {
        send_text(shared, &text);
    } else {
        lock(&shared.inner).pending.push(text);
    }
}

fn send_text(shared: &Arc<Shared>, text: &str) {
    let ws = lock(&shared.inner).ws.clone();
    if let Some(ws) = ws {
        let _ = ws.send_with_str(text);
    }
}

fn resubscribe_all(shared: &Arc<Shared>) {
    let subs: Vec<(String, Option<String>)> = lock(&shared.sub_filters)
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (collection, filter) in subs {
        let mut frame = json!({ "action": "subscribe", "collection": collection });
        if let Some(f) = filter {
            frame["filter"] = json!(f);
        }
        send_or_queue(shared, frame.to_string());
    }
    let presences: Vec<(String, Option<Value>)> = lock(&shared.presence_states)
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (channel, state) in presences {
        let mut frame = json!({ "action": "presence_join", "channel": channel });
        if let Some(s) = state {
            frame["state"] = s;
        }
        send_or_queue(shared, frame.to_string());
    }
}

fn dispatch(shared: &Arc<Shared>, text: &str) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(event) = RealtimeEvent::from_value(value) else {
        return;
    };
    for (_, h) in lock(&shared.handlers).iter() {
        h(event.clone());
    }
    match &event {
        RealtimeEvent::Record(rec) => {
            for key in [rec.collection.as_str(), "*"] {
                if let Some(h) = lock(&shared.subs).get(key) {
                    h(event.clone());
                }
            }
        }
        RealtimeEvent::Presence(pres) => {
            if let Some(h) = lock(&shared.presences).get(&pres.channel) {
                h(event.clone());
            }
        }
        RealtimeEvent::Raw(_) => {}
    }
}

impl std::fmt::Debug for RealtimeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealtimeClient")
            .field("connected", &self.connected())
            .finish_non_exhaustive()
    }
}
