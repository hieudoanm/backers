//! Backers — a single-binary Back-end as a Service.
//!
//! `backers` starts the HTTP server; `backers mcp serve` starts the MCP
//! server on stdio; `backers db …` converts DBML files. The servers share the
//! same SQLite database.

mod db;

use std::io::Write;

/// USAGE lists the accepted command lines. The default is the HTTP server, so
/// `backers` and `backers serve` both start it.
const USAGE: &str = "\
Backers - blazing fast Back-end as a Service

Usage:
  backers              Start the HTTP server
  backers serve        Start the HTTP server
  backers kv           Start the KV (Redis-style) TCP server
  backers kv --port N  KV server on port N (default 6379)
  backers db sql|mermaid|svg|all|parse [file.dbml] [-o out]
                       Convert DBML to SQL, Mermaid, SVG or a JSON summary
                       ('-' reads the schema from stdin)
  backers mcp serve    Start the MCP server on stdio
  backers mcp serve --kv HOST:PORT
                       MCP server proxying a remote backers kv server
  backers --help       Show this message
  backers --version    Show the version";

/// Reports whether the arguments select the stdio MCP server, with the
/// optional `--kv HOST:PORT` remote-store flag.
/// Parsed `backers mcp serve` flags.
struct McpFlags {
    kv: Option<String>,
    http: Option<String>,
    token: Option<String>,
}

/// Reports whether the arguments select the MCP server and parses its flags.
fn mcp_args(args: &[String]) -> Option<McpFlags> {
    if args.first().map(String::as_str) != Some("mcp")
        || args.get(1).map(String::as_str) != Some("serve")
    {
        return None;
    }
    let mut flags = McpFlags {
        kv: None,
        http: None,
        token: None,
    };
    let mut i = 2;
    while i < args.len() {
        let take = |i: &mut usize| -> Option<String> {
            let value = args.get(*i + 1)?.clone();
            *i += 2;
            Some(value)
        };
        match args[i].as_str() {
            "--kv" => flags.kv = Some(take(&mut i)?),
            "--http" => flags.http = Some(take(&mut i)?),
            "--token" => flags.token = Some(take(&mut i)?),
            _ => return None,
        }
    }
    Some(flags)
}

/// Parses `backers kv [--port N] [--bind ADDR]` into (bind, port).
fn kv_args(args: &[String]) -> Option<(String, u16)> {
    if args.first().map(String::as_str) != Some("kv") {
        return None;
    }
    let mut bind = "127.0.0.1".to_string();
    let mut port = 6379u16;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                port = args.get(i + 1)?.parse().ok()?;
                i += 2;
            }
            "--bind" => {
                bind = args.get(i + 1)?.clone();
                i += 2;
            }
            _ => return None,
        }
    }
    Some((bind, port))
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!(
            "backers {} (build {})",
            env!("CARGO_PKG_VERSION"),
            env!("BACKERS_BUILD_TIME")
        );
        return;
    }
    if let Some(flags) = mcp_args(&args) {
        // The MCP server is synchronous and reads stdin to EOF, so it runs on
        // this thread. It opens the same SQLite database as the HTTP server,
        // or proxies a remote `backers kv` listener when --kv is given.
        let mut stderr = std::io::stderr();
        let result = match flags.http {
            Some(addr) => backers_mcp::run_http(
                &addr,
                flags.token.or_else(|| std::env::var("MCP_AUTH_TOKEN").ok()),
                flags.kv,
            ),
            None => match flags.kv {
                Some(addr) => backers_mcp::run_remote(&addr, &mut stderr),
                None => backers_mcp::run(&mut stderr),
            },
        };
        if let Err(err) = result {
            let _ = writeln!(stderr, "backers mcp serve: {err}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(result) = db::db_args(&args) {
        match result {
            Ok(cmd) => {
                if let Err(err) = db::run(&cmd) {
                    eprintln!("backers db {}: {err}", cmd.command);
                    std::process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("backers db: {err}");
                std::process::exit(2);
            }
        }
        return;
    }
    if let Some((bind, port)) = kv_args(&args) {
        if let Err(err) = backers_kv::kvproto::serve_kv(&bind, port).await {
            eprintln!("backers kv: {err}");
            std::process::exit(1);
        }
        return;
    }
    // Bare `serve` and no arguments both start the HTTP server; anything else
    // is a typo worth reporting rather than silently ignoring.
    let starts_http = args.is_empty() || (args.len() == 1 && args[0] == "serve");
    if !starts_http {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }

    backers_server::app::serve_http().await
}
