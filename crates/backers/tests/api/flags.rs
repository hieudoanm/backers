use super::*;

fn flag_body(name: &str) -> Value {
    json!({
        "name": name,
        "enabled": true,
        "rules": [{"attribute": "plan", "op": "eq", "value": "pro"}],
        "payload": {"variant": "new-ui"},
        "description": "test flag"
    })
}

#[tokio::test]
async fn test_flags_crud_and_evaluate() {
    let svr = Svr::start().await;
    let token = svr.register().await;

    // admin CRUD
    let r = svr
        .post_auth("/api/flags", &token, flag_body("new-ui"))
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let flag: Value = r.json().await.unwrap();
    assert_eq!(flag["name"], "new-ui");
    assert_eq!(flag["enabled"], true);
    assert_eq!(flag["payload"]["variant"], "new-ui");

    let r = svr.get_auth("/api/flags", &token).await;
    let flags: Vec<Value> = r.json().await.unwrap();
    assert_eq!(flags.len(), 1);

    let r = svr.get_auth("/api/flags/new-ui", &token).await;
    assert_eq!(r.status(), StatusCode::OK);

    // patch: disable
    let r = svr
        .patch_auth("/api/flags/new-ui", &token, json!({"enabled": false}))
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let flag: Value = r.json().await.unwrap();
    assert_eq!(flag["enabled"], false);
    assert_eq!(flag["rules"].as_array().unwrap().len(), 1); // preserved

    // re-enable + evaluate: rule requires plan=pro
    let r = svr
        .patch_auth("/api/flags/new-ui", &token, json!({"enabled": true}))
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    // context without plan → off
    let r = svr
        .post(
            "/api/flags/evaluate",
            json!({"flags": ["new-ui"], "context": {"plan": "free"}}),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["flags"]["new-ui"]["enabled"], false);
    assert!(v["flags"]["new-ui"].get("payload").is_none());

    // context with plan=pro → on + payload
    let r = svr
        .post(
            "/api/flags/evaluate",
            json!({"flags": ["new-ui"], "context": {"plan": "pro"}}),
        )
        .await;
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["flags"]["new-ui"]["enabled"], true);
    assert_eq!(v["flags"]["new-ui"]["payload"]["variant"], "new-ui");

    // unknown flag is omitted from the result
    let r = svr
        .post("/api/flags/evaluate", json!({"flags": ["nope"]}))
        .await;
    let v: Value = r.json().await.unwrap();
    assert!(v["flags"].as_object().unwrap().is_empty());

    // empty flags list evaluates everything
    let r = svr.post("/api/flags/evaluate", json!({})).await;
    let v: Value = r.json().await.unwrap();
    assert!(v["flags"].get("new-ui").is_some());

    // auth attributes participate: rule on email domain
    let r = svr
        .post_auth(
            "/api/flags",
            &token,
            json!({
                "name": "staff-only",
                "enabled": true,
                "rules": [{"attribute": "email", "op": "ends_with", "value": "@test.com"}]
            }),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    // no token → email attribute missing → neq-style ops still work but eq fails
    let r = svr
        .post("/api/flags/evaluate", json!({"flags": ["staff-only"]}))
        .await;
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["flags"]["staff-only"]["enabled"], false);

    // with admin token (admin@test.com) → matches
    let r = svr
        .post_auth(
            "/api/flags/evaluate",
            &token,
            json!({"flags": ["staff-only"]}),
        )
        .await;
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["flags"]["staff-only"]["enabled"], true);

    // non-admin cannot manage flags
    let r = svr
        .post(
            "/api/auth/register",
            json!({"email": "user@test.com", "password": "user123"}),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = svr
        .post(
            "/api/auth/login",
            json!({"email": "user@test.com", "password": "user123"}),
        )
        .await;
    let login: Value = r.json().await.unwrap();
    let user_token = login["token"].as_str().unwrap().to_string();
    let r = svr.get_auth("/api/flags", &user_token).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    // …but can evaluate
    let r = svr
        .post_auth(
            "/api/flags/evaluate",
            &user_token,
            json!({"flags": ["new-ui"], "context": {"plan": "pro"}}),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    // delete
    let r = svr.delete_auth("/api/flags/new-ui", &token).await;
    assert_eq!(r.status(), StatusCode::NO_CONTENT);
    let r = svr.get_auth("/api/flags/new-ui", &token).await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_flags_rule_operators() {
    let svr = Svr::start().await;
    let token = svr.register().await;

    // string operators
    for (op, attr_val, rule_val, expected) in [
        ("eq", "pro", "pro", true),
        ("eq", "pro", "free", false),
        ("neq", "pro", "free", true),
        ("contains", "production", "prod", true),
        ("starts_with", "production", "pro", true),
        ("ends_with", "production", "tion", true),
    ] {
        let r = svr
            .post_auth(
                "/api/flags",
                &token,
                json!({"name": format!("op-{op}-{expected}"), "enabled": true,
                       "rules": [{"attribute": "plan", "op": op, "value": rule_val}]}),
            )
            .await;
        assert_eq!(r.status(), StatusCode::OK);
        let name = format!("op-{op}-{expected}");
        let r = svr
            .post(
                "/api/flags/evaluate",
                json!({"flags": [name], "context": {"plan": attr_val}}),
            )
            .await;
        let v: Value = r.json().await.unwrap();
        assert_eq!(
            v["flags"][name.as_str()]["enabled"],
            expected,
            "{op} {attr_val} vs {rule_val}"
        );
    }

    // numeric comparisons
    let r = svr
        .post_auth(
            "/api/flags",
            &token,
            json!({"name": "big-spender", "enabled": true,
                   "rules": [{"attribute": "spent", "op": "gte", "value": 100}]}),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);

    for (spent, expected) in [(150, true), (99, false)] {
        let r = svr
            .post(
                "/api/flags/evaluate",
                json!({"flags": ["big-spender"], "context": {"spent": spent}}),
            )
            .await;
        let v: Value = r.json().await.unwrap();
        assert_eq!(
            v["flags"]["big-spender"]["enabled"], expected,
            "spent={spent}"
        );
    }

    // `in` operator
    let r = svr
        .post_auth(
            "/api/flags",
            &token,
            json!({"name": "beta", "enabled": true,
                   "rules": [{"attribute": "channel", "op": "in", "value": ["beta", "alpha"]}]}),
        )
        .await;
    assert_eq!(r.status(), StatusCode::OK);
    let r = svr
        .post(
            "/api/flags/evaluate",
            json!({"flags": ["beta"], "context": {"channel": "beta"}}),
        )
        .await;
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["flags"]["beta"]["enabled"], true);
}
