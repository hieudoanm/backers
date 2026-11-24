# How to install the backers CLI on macOS (Apple Silicon)

For Macs with an M-series chip (arm64). The single-binary Back-end as a Service

## Option 1: install script (recommended)

```sh
curl -fsSL https://github.com/hieudoanm/backers/master/scripts/install.sh | bash
```

The script detects `macos/arm64` and downloads
`cli-backers-aarch64-apple-darwin` from the
[latest release](https://github.com/hieudoanm/backers/releases/latest). It
installs to `~/.local/bin/backers` — override with `INSTALL_DIR=/path`.

## Option 2: manual download

1. Open the [downloads page](https://hieudoanm.github.io/backers/downloads/) and
   download **macOS (Silicon)** under **CLI** —
   `cli-backers-aarch64-apple-darwin`.
2. Make it executable and move it onto your `PATH`:

```sh
chmod +x cli-backers-aarch64-apple-darwin
mv cli-backers-aarch64-apple-darwin ~/.local/bin/backers
```

3. Verify:

```sh
backers --version
```

## Troubleshooting

- `backers: command not found` — the install directory is not on your `PATH`.
  Add `export PATH="$HOME/.local/bin:$PATH"` to your shell profile.
- Gatekeeper warning — the binary is unsigned. Right-click → Open, or run
  `xattr -d com.apple.quarantine` on the installed binary.
