.PHONY: build test fmt lint clean install integration-test

build:
	cargo build --workspace --exclude iad-grpc --exclude iad-wasm

release:
	cargo build --release --workspace --exclude iad-grpc --exclude iad-wasm

test:
	cargo test --workspace --exclude iad-grpc --exclude iad-wasm
	node --test sdk/node/src/protocol.test.js

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --exclude iad-grpc --exclude iad-wasm -- -D warnings

install: release
	cp target/release/iad ~/.local/bin/

clean:
	cargo clean

integration-test:
	bash tests/integration/agent_lifecycle.sh

audit:
	cargo audit
