## Blazers — latest build

Blazers is a single-binary Backend-as-a-Service written in Rust: auth, a
schema-less document database on SQLite, file buckets, realtime
(WebSocket + SSE), webhooks, cron jobs, an encrypted secrets vault,
cache, pub/sub, notifications, an embedded admin dashboard, OpenAPI docs
and an MCP server.

### Download

| Platform | Asset |
| --- | --- |
| Linux x86_64 | `blazers-x86_64-unknown-linux-gnu` |
| macOS Apple Silicon | `blazers-aarch64-apple-darwin` |
| Windows x86_64 | `blazers-x86_64-pc-windows-msvc.exe` |

Or use the published Docker image (`ghcr.io/hieudoanm/blazers`).

### Quick start

```sh
blazers serve          # HTTP API + dashboard on :8090
blazers mcp serve      # MCP server over stdio
```

Data lives in `$BLAZERS_DATA` (default `~/.blazers`).

### Documentation

- [README](https://github.com/hieudoanm/blazers#readme)
- [Docs site](https://hieudoanm.github.io/blazers/docs/)
