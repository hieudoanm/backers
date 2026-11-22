.PHONY: all fmt fmt-check clippy test build check run clean

all: fmt-check clippy test build

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --release -p backers

check:
	cargo check --workspace --all-targets

run:
	cargo run -p backers --release -- serve

mcp:
	cargo run -p backers --release -- mcp serve

clean:
	cargo clean
	rm -rf target
