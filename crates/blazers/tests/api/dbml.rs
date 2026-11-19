use super::*;

const SCHEMA: &str = "Table users {\n  id integer [pk, increment]\n  email varchar(255) [unique, not null]\n}\n\nTable posts {\n  id integer [pk]\n  user_id integer [ref: > users.id]\n}\n\nRef: user_posts users.id < posts.user_id\n";

#[tokio::test]
async fn test_dbml_convert_endpoints() {
    let svr = Svr::start().await;
    let body = json!({"dbml": SCHEMA, "formats": ["sql", "mermaid", "svg"]});

    let r = svr.post("/api/dbml/convert", body.clone()).await;
    assert_eq!(r.status(), StatusCode::OK);
    let v = r.json::<Value>().await.unwrap();
    assert!(v["sql"].as_str().unwrap().contains("CREATE TABLE users"));
    assert!(v["mermaid"].as_str().unwrap().contains("erDiagram"));
    assert!(v["svg"].as_str().unwrap().contains("<svg"));

    // default formats are sql + mermaid only
    let r = svr.post("/api/dbml/convert", json!({"dbml": SCHEMA})).await;
    let v = r.json::<Value>().await.unwrap();
    assert!(v.get("sql").is_some());
    assert!(v.get("mermaid").is_some());
    assert!(v.get("svg").is_none());

    let r = svr.post("/api/dbml/convert/sql", body.clone()).await;
    let v = r.json::<Value>().await.unwrap();
    assert!(v["sql"].as_str().unwrap().contains("FOREIGN KEY"));
    assert!(v.get("mermaid").is_none());

    let r = svr.post("/api/dbml/convert/mermaid", body).await;
    let v = r.json::<Value>().await.unwrap();
    assert!(
        v["mermaid"]
            .as_str()
            .unwrap()
            .contains("users ||--o{ posts")
    );

    let r = svr.post("/api/dbml/parse", json!({"dbml": SCHEMA})).await;
    assert_eq!(r.status(), StatusCode::OK);
    let v = r.json::<Value>().await.unwrap();
    let summary: Value = serde_json::from_str(v["summary"].as_str().unwrap()).unwrap();
    assert_eq!(summary["tables"][0]["name"], "users");
}

#[tokio::test]
async fn test_dbml_convert_errors() {
    let svr = Svr::start().await;

    let r = svr.post("/api/dbml/convert", json!({})).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);

    let r = svr
        .post("/api/dbml/convert", json!({"dbml": "not dbml {{{"}))
        .await;
    // Permissive parser: unknown lines are ignored, so this still succeeds.
    assert_eq!(r.status(), StatusCode::OK);
}

#[test]
fn test_dbml_cli_subcommand() {
    let dir = std::env::temp_dir().join(format!("blazers-dbml-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("schema.dbml");
    let output = dir.join("out.sql");
    std::fs::write(&input, SCHEMA).unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_blazers"))
        .args(["db", "sql"])
        .arg(&input)
        .args(["-o"])
        .arg(&output)
        .status()
        .expect("run blazers db sql");
    assert!(status.success());
    let sql = std::fs::read_to_string(&output).unwrap();
    assert!(sql.contains("CREATE TABLE users"));
    assert!(sql.contains("ALTER TABLE posts ADD CONSTRAINT user_posts"));

    // `db all` writes the three default artifacts into the directory.
    let out_dir = dir.join("all");
    let status = Command::new(env!("CARGO_BIN_EXE_blazers"))
        .args(["db", "all"])
        .arg(&input)
        .args(["-d"])
        .arg(&out_dir)
        .status()
        .expect("run blazers db all");
    assert!(status.success());
    for name in ["initial_schema.sql", "database-erd.mmd", "schema.svg"] {
        assert!(out_dir.join(name).exists(), "missing {name}");
    }

    // stdin via `-`
    let mut child = Command::new(env!("CARGO_BIN_EXE_blazers"))
        .args(["db", "parse", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn blazers db parse");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(SCHEMA.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"users\""));
}
