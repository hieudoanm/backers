//! Integration tests against a canned HTTP/1.1 server — exercises URL
//! building, headers, JSON bodies and error mapping without a real Blazers.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::mpsc as std_mpsc;

use blazers_sdk::types::{LoginResult, RealtimeEvent};
use blazers_sdk::{Blazers, BlazersError, ClientOptions, ListOptions};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Spawns a server that captures each raw request and replies with `body`.
/// Returns (base_url, receiver of raw request strings).
async fn mock_server(
    status: u16,
    body: &'static str,
) -> (
    String,
    tokio::task::JoinHandle<()>,
    std_mpsc::Receiver<String>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = std_mpsc::channel::<String>();
    let handle = tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut buf = Vec::with_capacity(8192);
                let mut chunk = [0u8; 4096];
                // Read until end of headers + declared body.
                loop {
                    match sock.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&chunk[..n]);
                            if request_complete(&buf) {
                                break;
                            }
                        }
                    }
                }
                let _ = tx.send(String::from_utf8_lossy(&buf).to_string());
                let reason = if status == 200 { "OK" } else { "ERR" };
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
            });
        }
    });
    (format!("http://{addr}"), handle, rx)
}

fn request_complete(buf: &[u8]) -> bool {
    let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&buf[..pos]);
    let len = head
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse::<usize>().ok())
        })
        .unwrap_or(0);
    buf.len() >= pos + 4 + len
}

fn client(url: &str) -> Blazers {
    Blazers::new(ClientOptions {
        url: url.to_string(),
        ..Default::default()
    })
}

#[tokio::test]
async fn get_parses_json_and_sends_headers() {
    static BODY: &str = r#"{"name":"posts","schema":"{}","list_rule":"","view_rule":"","create_rule":"","update_rule":"","delete_rule":"","type":"records","created_at":"t","updated_at":"t"}"#;
    let (url, _h, rx) = mock_server(200, BODY).await;
    let bz = Blazers::new(ClientOptions {
        url: url.clone(),
        token: Some("tok123".into()),
        ..Default::default()
    });
    let col: blazers_sdk::Collection = bz.collections.get("posts").await.unwrap();
    assert_eq!(col.name, "posts");
    let req = rx.recv().unwrap();
    assert!(
        req.starts_with("GET /api/collections/posts HTTP/1.1"),
        "{req}"
    );
    assert!(
        req.to_ascii_lowercase()
            .contains("authorization: bearer tok123"),
        "{req}"
    );
}

#[tokio::test]
async fn non_2xx_maps_to_blazers_error() {
    static BODY: &str = r#"{"error":"collection not found"}"#;
    let (url, _h, _rx) = mock_server(404, BODY).await;
    let bz = client(&url);
    let err = bz.collections.get("nope").await.unwrap_err();
    match err {
        BlazersError::Http {
            status, message, ..
        } => {
            assert_eq!(status, 404);
            assert_eq!(message, "collection not found");
        }
        other => panic!("expected Http error, got {other}"),
    }
}

#[tokio::test]
async fn login_stores_token_for_later_requests() {
    static LOGIN: &str = r#"{"user":{"id":"u1","email":"a@b.c","data":{},"verified":false,"mfa_enabled":false,"created_at":"t","updated_at":"t"},"token":"sekrit"}"#;
    let (url, _h, rx) = mock_server(200, LOGIN).await;
    let bz = client(&url);
    let res = bz
        .auth
        .login(&blazers_sdk::Credentials {
            email: "a@b.c".into(),
            password: "pw".into(),
        })
        .await
        .unwrap();
    assert!(matches!(res, LoginResult::Session(_)));
    assert_eq!(bz.http.auth_token().as_deref(), Some("sekrit"));

    // Next request carries the bearer token.
    let _: Value = bz.http.get("/api/auth/me").await.unwrap_or(Value::Null);
    let req = rx.recv().unwrap(); // login request
    let req2 = rx.recv().unwrap(); // me request
    assert!(req2.contains("authorization: Bearer sekrit"), "{req2}");
    let _ = req;
}

#[tokio::test]
async fn record_list_builds_query_string() {
    static BODY: &str = r#"{"records":[],"total":0,"page":2,"per_page":10,"total_pages":0}"#;
    let (url, _h, rx) = mock_server(200, BODY).await;
    let bz = client(&url);
    let opts = ListOptions {
        page: Some(2),
        per_page: Some(10),
        filter: Some("status=\"live\"".into()),
        sort: Some("-created_at".into()),
        ..Default::default()
    };
    let page = bz.collection("posts").list(&opts).await.unwrap();
    assert_eq!(page.page, 2);
    let req = rx.recv().unwrap();
    let line = req.lines().next().unwrap();
    assert!(line.contains("/api/collections/posts/records?"), "{line}");
    assert!(line.contains("page=2"), "{line}");
    assert!(line.contains("per_page=10"), "{line}");
    assert!(line.contains("filter="), "{line}");
    assert!(line.contains("sort=-created_at"), "{line}");
}

#[tokio::test]
async fn batch_posts_requests_envelope() {
    static BODY: &str = r#"[{"status":200,"body":{"ok":true}}]"#;
    let (url, _h, rx) = mock_server(200, BODY).await;
    let bz = client(&url);
    let results = bz
        .batch(vec![blazers_sdk::BatchSubRequest {
            method: "GET".into(),
            url: "/api/health".into(),
            headers: None,
            body: None,
        }])
        .await
        .unwrap();
    assert_eq!(results[0].status, 200);
    let req = rx.recv().unwrap();
    assert!(req.contains("POST /api/batch"), "{req}");
    assert!(req.contains("\"requests\""), "{req}");
}

#[tokio::test]
async fn flags_evaluate_posts_context() {
    static BODY: &str = r#"{"flags":{"f":{"enabled":true,"payload":{"v":1}}}}"#;
    let (url, _h, rx) = mock_server(200, BODY).await;
    let bz = client(&url);
    let mut ctx = std::collections::HashMap::new();
    ctx.insert("plan".to_string(), json!("pro"));
    let res = bz.flags.evaluate(&["f".to_string()], &ctx).await.unwrap();
    assert!(res["f"].enabled);
    assert_eq!(res["f"].payload, Some(json!({"v": 1})));
    let req = rx.recv().unwrap();
    assert!(req.contains("POST /api/flags/evaluate"), "{req}");
    assert!(req.contains("\"plan\":\"pro\""), "{req}");
}

#[test]
fn login_result_untagged_variants() {
    let session: LoginResult = serde_json::from_value(json!({
        "user": {"id":"u","email":"e","data":{},"verified":false,"mfa_enabled":false,"created_at":"t","updated_at":"t"},
        "token": "tok"
    }))
    .unwrap();
    assert!(session.token().is_some());

    let challenge: LoginResult = serde_json::from_value(json!({
        "mfa_required": true,
        "user": {"id":"u","email":"e","data":{},"verified":false,"mfa_enabled":true,"created_at":"t","updated_at":"t"}
    }))
    .unwrap();
    assert!(challenge.token().is_none());
}

#[test]
fn realtime_event_routes_by_type() {
    let rec = json!({
        "type": "record", "action": "create", "collection": "posts",
        "record": {"id":"r1","data":{},"created_at":"t","updated_at":"t"}
    });
    match RealtimeEvent::from_value(rec).unwrap() {
        RealtimeEvent::Record(e) => {
            assert_eq!(e.collection, "posts");
            assert_eq!(e.record.id, "r1");
        }
        _ => panic!("expected record event"),
    }
    let pres = json!({"type":"presence","action":"join","channel":"c","client":"x"});
    assert!(matches!(
        RealtimeEvent::from_value(pres).unwrap(),
        RealtimeEvent::Presence(_)
    ));
    let ping = json!({"type":"ping"});
    assert!(matches!(
        RealtimeEvent::from_value(ping).unwrap(),
        RealtimeEvent::Raw(_)
    ));
    assert!(RealtimeEvent::from_value(json!({"no_type":1})).is_none());
}

#[test]
fn url_and_ws_url_building() {
    let bz = client("https://example.com/blazers/");
    assert_eq!(
        bz.http.url("/api/health"),
        "https://example.com/blazers/api/health"
    );
    assert_eq!(
        bz.http.url("api/health"),
        "https://example.com/blazers/api/health"
    );
    assert_eq!(bz.http.ws_url(), "wss://example.com/blazers/ws");
    let local = client("http://localhost:8090");
    assert_eq!(local.http.ws_url(), "ws://localhost:8090/ws");
}
