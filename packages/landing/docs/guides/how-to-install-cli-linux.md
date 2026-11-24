# How to install the backers CLI on Linux

For x86_64 Linux distributions. The single-binary Back-end as a Service

## Option 1: install script (recommended)

```sh
curl -fsSL https://github.com/hieudoanm/backers/master/scripts/install.sh | bash
```

The script detects `linux/x86_64` and downloads
`cli-backers-x86_64-unknown-linux-gnu` from the
[latest release](https://github.com/hieudoanm/backers/releases/latest). It
installs to `~/.local/bin/backers` — override with `INSTALL_DIR=/path`.

## Option 2: manual download

1. Open the [downloads page](https://hieudoanm.github.io/backers/downloads/) and
   download **Linux** under **CLI** — `cli-backers-x86_64-unknown-linux-gnu`.
2. Make it executable and move it onto your `PATH`:

```sh
chmod +x cli-backers-x86_64-unknown-linux-gnu
mv cli-backers-x86_64-unknown-linux-gnu ~/.local/bin/backers
```

3. Verify:

```sh
backers --version
```

## Troubleshooting

- `backers: command not found` — ensure `~/.local/bin` is on your `PATH` (most
  distributions already include it).
- `No such file or directory` on a glibc system — the binary is built against
  glibc; on musl-based distros (Alpine) build from source instead.
