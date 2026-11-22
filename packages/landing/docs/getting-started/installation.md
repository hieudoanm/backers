# Installation

Backers ships as a single static binary. There is no runtime dependency, no
database server to provision, and no build step required on the target machine.

## Prebuilt binaries

Prebuilt binaries for Linux (x86_64), macOS (Apple Silicon), and Windows
(x86_64) are attached to each
[GitHub Release](https://github.com/hieudoanm/backers/releases).

```sh
# macOS / Linux
curl -L -o backers https://github.com/hieudoanm/backers/releases/latest/download/backers-<target>
chmod +x backers
./backers serve
```

## Build from source

Requires a recent stable Rust toolchain (edition 2024).

```sh
cargo build --release -p backers
./target/release/backers serve
```

The workspace is split into four crates:

| Crate            | Purpose                                           |
| ---------------- | ------------------------------------------------- |
| `backers-core`   | SQLite schema, migrations, models, wire types     |
| `backers-server` | HTTP API, realtime, admin dashboard, all features |
| `backers-mcp`    | MCP stdio server exposing Backers as agent tools  |
| `backers`        | Thin binary: `serve` (HTTP) and `mcp serve`       |

## Commands

```text
backers serve        # start the HTTP server + admin dashboard (default)
backers kv           # start the Redis-style KV TCP server (port 6379)
backers mcp serve    # start the MCP stdio server
backers --help       # usage
```

## Configuration

All configuration is via environment variables. Everything has a working
default, so `backers serve` is enough to get started.

| Variable       | Default                           | Purpose                                    |
| -------------- | --------------------------------- | ------------------------------------------ |
| `PORT`         | `8080`                            | HTTP listen port                           |
| `BACKERS_DATA` | `~/.backers`                      | Data directory (SQLite DB, uploaded files) |
| `JWT_SECRET`   | `dev-secret-change-in-production` | JWT signing secret — **set in production** |

The secrets-vault encryption key (AES-256-GCM) is generated automatically on
first run and stored inside `BACKERS_DATA`. Back up that directory to keep your
secrets decryptable.

## Verify the install

```sh
curl http://localhost:8080/api/health
# {"status":"ok"}
```

Then open <http://localhost:8080/> for the admin dashboard.
