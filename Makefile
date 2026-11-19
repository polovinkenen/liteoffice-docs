.PHONY: install dev build lint typecheck test test-rust fmt fmt-check clippy deny check

install:
	npm ci

dev:
	npm run dev:desktop

build:
	npm run build:desktop

lint:
	npm run lint

typecheck:
	npm run typecheck

test:
	npm run test

test-rust:
	cargo test --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

deny:
	cargo deny check -W unmaintained

check: lint typecheck test test-rust fmt-check clippy deny
