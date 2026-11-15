.PHONY: install dev build lint typecheck test fmt fmt-check clippy check

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

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

check: lint typecheck test fmt-check clippy
