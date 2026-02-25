default: build

all: test

test: build
	cargo test

test-e2e: build
	@echo "Running E2E upgrade and migration tests..."
	cargo test test_e2e_upgrade_with_pause -- --nocapture --test-threads=1

test-e2e-verbose: build
	@echo "Running E2E tests with verbose output..."
	cargo test test_e2e_upgrade_with_pause -- --nocapture --test-threads=1 --show-output

test-pause: build
	@echo "Running pause/resume tests..."
	cargo test test_pause -- --nocapture

test-emergency: build
	@echo "Running emergency withdraw tests..."
	cargo test emergency_withdraw -- --nocapture

test-upgrade-suite: test-e2e test-pause test-emergency
	@echo "✅ All upgrade-related tests completed"

build:
	stellar contract build
	@ls -l target/wasm32v1-none/release/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean

