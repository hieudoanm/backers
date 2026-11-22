# MCP server

`backers mcp serve` runs a
[Model Context Protocol](https://modelcontextprotocol.io/) stdio server that
exposes your Backers database as agent tools — the same data the HTTP API
serves, without HTTP credentials.

## Run it

```sh
backers mcp serve
```

It reads the same `BACKERS_DATA` directory as `backers serve`, so it can run
alongside a live server (SQLite WAL handles concurrent readers).

## Tools

| Tool                         | Does                                           |
| ---------------------------- | ---------------------------------------------- |
| `backers_health`             | liveness / status probe                        |
| `backers_collections_list`   | list collections + schemas                     |
| `backers_collections_create` | create a collection                            |
| `backers_collections_delete` | delete a collection                            |
| `backers_records_list`       | paginated record listing                       |
| `backers_records_get`        | fetch one record                               |
| `backers_records_create`     | create a record                                |
| `backers_records_update`     | update a record                                |
| `backers_records_delete`     | delete a record                                |
| `backers_export`             | dump collections/records/buckets/files as JSON |
| `backers_import`             | import an export payload                       |
| `backers_flags_list`         | list feature flags                             |
| `backers_flags_get`          | fetch one flag                                 |
| `backers_flags_upsert`       | create or replace a flag                       |
| `backers_flags_delete`       | delete a flag                                  |
| `backers_flags_evaluate`     | evaluate flags against a context object        |
| `backers_dbml_sql`           | DBML → PostgreSQL DDL                          |
| `backers_dbml_mermaid`       | DBML → Mermaid ER diagram                      |
| `backers_dbml_svg`           | DBML → SVG ER diagram                          |
| `backers_dbml_parse`         | DBML → JSON summary                            |

Tool names are prefixed `backers_` (rebranded from the original `backbone_*`).

## Wiring into an MCP client

Example `mcp.json` / client config:

```json
{
  "mcpServers": {
    "backers": {
      "command": "/path/to/backers",
      "args": ["mcp", "serve"],
      "env": { "BACKERS_DATA": "/path/to/data" }
    }
  }
}
```

## Remote KV mode

`backers mcp serve --kv HOST:PORT` proxies a remote
[`backers kv`](../api-reference/cache.md#kv-tcp-protocol) listener instead of
opening the local database — the same pattern as kevin's `mcp serve --addr`. The
tool surface becomes `backers_kv_*` (ping, set, get, del, exists, keys, len,
ttl, expire) and each call speaks the inline TCP protocol, so the MCP server
needs no database of its own. Set `BACKERS_KV_PASSWORD` when the remote listener
requires `AUTH`.

```json
{
  "mcpServers": {
    "backers-kv": {
      "command": "/path/to/backers",
      "args": ["mcp", "serve", "--kv", "cache.internal:6379"]
    }
  }
}
```

## Design notes

- Stdio transport only — no network listener, no auth handshake needed. `--kv`
  mode is the exception: it is a _client_ of the remote KV port.
- The MCP crate (`crates/backers-mcp`) depends on `backers-core` and
  `backers-server` for shared models and handlers; tool implementations live in
  `crates/backers-mcp/src/tools.rs`.
- Because it shares the database file, MCP writes are immediately visible over
  HTTP (and vice versa) — no sync layer needed.
