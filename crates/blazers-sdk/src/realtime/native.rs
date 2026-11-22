//! Native realtime — `tokio-tungstenite` over `/ws`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Notify, mpsc};
use tokio_tungstenite::tungstenite::Message;

use super::*;
use crate::http::HttpClient;

type Handler = Arc<dyn Fn(RealtimeEvent) + Send + Sync>;

struct Shared {
    /// collection (or "*") → record-event handler.
    subs: Mutex<HashMap<String, Handler>>,
    /// channel → presence-event handler.
    presences: Mutex<HashMap<String, Handler>>,
    /// handlers for every frame.
    handlers: Mutex<Vec<Handler>>,
    /// client → server text frames for the live socket.
    tx: Mutex<Option<mpsc::UnboundedSender<String>>>,
    connected: AtomicBool,
    closed: AtomicBool,
    /// filters captured at subscribe time for resubscribe.
    sub_filters: Mutex<HashMap<String, Option<String>>>,
    presence_states: Mutex<HashMap<String, Option<Value>>>,
    backoff_step: Mutex<u32>,
    connect_lock: tokio::sync::Mutex<()>,
    ready: Notify,
}

/// WebSocket realtime client. Cheap to clone; shares one connection.
#[derive(Clone)]
pub struct RealtimeClient {
    http: HttpClient,
    shared: Arc<Shared>,
}

impl RealtimeClient {
    pub(crate) fn new(http: HttpClient) -> Self {
        RealtimeClient {
            http,
            shared: Arc::new(Shared {
                subs: Mutex::new(HashMap::new()),
                presences: Mutex::new(HashMap::new()),
                handlers: Mutex::new(Vec::new()),
                tx: Mutex::new(None),
                connected: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                sub_filters: Mutex::new(HashMap::new()),
                presence_states: Mutex::new(HashMap::new()),
                backoff_step: Mutex::new(0),
                connect_lock: tokio::sync::Mutex::new(()),
                ready: Notify::new(),
            }),
        }
    }

    /// Whether the socket is currently open.
    pub fn connected(&self) -> bool {
        self.shared.connected.load(Ordering::SeqCst)
    }

    /// Connects (or waits for the in-flight connect). Returns once the
    /// socket is open, or after ~10s if the server never answers.
    pub async fn connect(&self) -> Result<()> {
        if self.connected() {
            return Ok(());
        }
        let _guard = self.shared.connect_lock.lock().await;
        if self.connected() {
            return Ok(());
        }
        self.shared.closed.store(false, Ordering::SeqCst);
        let shared = Arc::clone(&self.shared);
        let url = self.http.ws_url();
        tokio::spawn(async move { run_loop(shared, url).await });
        // Wait for the open event (or give up quietly — sends queue anyway).
        for _ in 0..100 {
            if self.connected() {
                return Ok(());
            }
            let _ = tokio::time::timeout(Duration::from_millis(100), self.shared.ready.notified())
                .await;
        }
        Ok(())
    }

    /// Subscribe to record events for `collection` (`"*"` = all collections).
    /// `on_event` fires for matching record frames.
    pub async fn subscribe(
        &self,
        collection: &str,
        filter: Option<&str>,
        on_event: impl Fn(RealtimeEvent) + Send + Sync + 'static,
    ) -> Result<()> {
        let handler: Handler = Arc::new(on_event);
        self.shared
            .subs
            .lock()
            .map_err(|_| crate::error::BlazersError::Transport("lock poisoned".into()))?
            .insert(collection.to_string(), handler);
        self.shared
            .sub_filters
            .lock()
            .map_err(|_| crate::error::BlazersError::Transport("lock poisoned".into()))?
            .insert(collection.to_string(), filter.map(str::to_string));
        self.connect().await?;
        let mut frame = json!({ "action": "subscribe", "collection": collection });
        if let Some(f) = filter {
            frame["filter"] = json!(f);
        }
        self.send_text(frame.to_string());
        Ok(())
    }

    /// Removes a subscription.
    pub async fn unsubscribe(&self, collection: &str) -> Result<()> {
        if let Ok(mut subs) = self.shared.subs.lock() {
            subs.remove(collection);
        }
        if let Ok(mut filters) = self.shared.sub_filters.lock() {
            filters.remove(collection);
        }
        if self.connected() {
            self.send_text(
                json!({ "action": "unsubscribe", "collection": collection }).to_string(),
            );
        }
        Ok(())
    }

    /// Join a presence channel (or update state with [`presence_update`](Self::presence_update)).
    pub async fn presence_join(
        &self,
        channel: &str,
        state: Option<Value>,
        on_event: impl Fn(RealtimeEvent) + Send + Sync + 'static,
    ) -> Result<()> {
        let handler: Handler = Arc::new(on_event);
        self.shared
            .presences
            .lock()
            .map_err(|_| crate::error::BlazersError::Transport("lock poisoned".into()))?
            .insert(channel.to_string(), handler);
        self.shared
            .presence_states
            .lock()
            .map_err(|_| crate::error::BlazersError::Transport("lock poisoned".into()))?
            .insert(channel.to_string(), state.clone());
        self.connect().await?;
        let mut frame = json!({ "action": "presence_join", "channel": channel });
        if let Some(s) = state {
            frame["state"] = s;
        }
        self.send_text(frame.to_string());
        Ok(())
    }

    /// Update presence state on a joined channel.
    pub async fn presence_update(&self, channel: &str, state: Value) -> Result<()> {
        if self.connected() {
            self.send_text(
                json!({ "action": "presence", "channel": channel, "state": state }).to_string(),
            );
        }
        Ok(())
    }

    /// Leave a presence channel.
    pub async fn presence_leave(&self, channel: &str) -> Result<()> {
        if let Ok(mut presences) = self.shared.presences.lock() {
            presences.remove(channel);
        }
        if let Ok(mut states) = self.shared.presence_states.lock() {
            states.remove(channel);
        }
        if self.connected() {
            self.send_text(json!({ "action": "presence_leave", "channel": channel }).to_string());
        }
        Ok(())
    }

    /// Raw frame listener for frames not covered by subscribe/presence.
    /// Returns nothing — the handler lives until [`close`](Self::close).
    pub fn on_message(&self, handler: impl Fn(RealtimeEvent) + Send + Sync + 'static) {
        if let Ok(mut handlers) = self.shared.handlers.lock() {
            handlers.push(Arc::new(handler));
        }
    }

    /// Permanently closes the socket (no reconnect).
    pub fn close(&self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        if let Ok(mut tx) = self.shared.tx.lock() {
            *tx = None; // dropping the sender ends the socket task's select loop
        }
        self.shared.connected.store(false, Ordering::SeqCst);
    }

    fn send_text(&self, text: String) {
        if let Ok(tx) = self.shared.tx.lock()
            && let Some(sender) = tx.as_ref()
        {
            let _ = sender.send(text);
        }
    }
}

async fn run_loop(shared: Arc<Shared>, url: String) {
    let mut step = 0u32;
    while !shared.closed.load(Ordering::SeqCst) {
        if let Ok((ws, _)) = tokio_tungstenite::connect_async(&url).await {
            if let Ok(mut s) = shared.backoff_step.lock() {
                *s = 0;
            }
            step = 0;
            let (mut sink, mut stream) = ws.split();
            let (tx, mut rx) = mpsc::unbounded_channel::<String>();
            if let Ok(mut slot) = shared.tx.lock() {
                *slot = Some(tx);
            }
            shared.connected.store(true, Ordering::SeqCst);
            shared.ready.notify_waiters();
            resubscribe_all(&shared);

            loop {
                tokio::select! {
                    outbound = rx.recv() => {
                        match outbound {
                            Some(text) => {
                                if sink.send(Message::Text(text.into())).await.is_err() {
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    inbound = stream.next() => {
                        match inbound {
                            Some(Ok(Message::Text(text))) => dispatch(&shared, &text),
                            Some(Ok(Message::Ping(p))) => {
                                let _ = sink.send(Message::Pong(p)).await;
                            }
                            Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                            _ => {}
                        }
                    }
                }
            }
            shared.connected.store(false, Ordering::SeqCst);
            if let Ok(mut slot) = shared.tx.lock() {
                *slot = None;
            }
        }
        if shared.closed.load(Ordering::SeqCst) {
            break;
        }
        let delay_ms = 1000u64 * 2u64.saturating_pow(step.min(4));
        step += 1;
        let _ = shared.backoff_step.lock().map(|mut s| *s = step);
        tokio::time::sleep(Duration::from_millis(delay_ms.min(15000))).await;
    }
    shared.connected.store(false, Ordering::SeqCst);
}

fn resubscribe_all(shared: &Shared) {
    let subs: Vec<(String, Option<String>)> = shared
        .sub_filters
        .lock()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    for (collection, filter) in subs {
        let mut frame = json!({ "action": "subscribe", "collection": collection });
        if let Some(f) = filter {
            frame["filter"] = json!(f);
        }
        send_frame(shared, frame);
    }
    let presences: Vec<(String, Option<Value>)> = shared
        .presence_states
        .lock()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    for (channel, state) in presences {
        let mut frame = json!({ "action": "presence_join", "channel": channel });
        if let Some(s) = state {
            frame["state"] = s;
        }
        send_frame(shared, frame);
    }
}

fn send_frame(shared: &Shared, frame: Value) {
    if let Ok(tx) = shared.tx.lock()
        && let Some(sender) = tx.as_ref()
    {
        let _ = sender.send(frame.to_string());
    }
}

fn dispatch(shared: &Shared, text: &str) {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return;
    };
    let Some(event) = RealtimeEvent::from_value(value) else {
        return;
    };
    if let Ok(handlers) = shared.handlers.lock() {
        for h in handlers.iter() {
            h(event.clone());
        }
    }
    match &event {
        RealtimeEvent::Record(rec) => {
            for key in [rec.collection.as_str(), "*"] {
                if let Ok(subs) = shared.subs.lock()
                    && let Some(h) = subs.get(key)
                {
                    h(event.clone());
                }
            }
        }
        RealtimeEvent::Presence(pres) => {
            if let Ok(presences) = shared.presences.lock()
                && let Some(h) = presences.get(&pres.channel)
            {
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
