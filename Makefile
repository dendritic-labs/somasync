# SomaSync Makefile
# Neural mesh networking library for distributed threat intelligence

.PHONY: test build clean release load-test-small load-test-medium load-test-large load-test-million

# Development targets
test:
	cargo test

build:
	cargo build

release:
	cargo build --release

clean:
	cargo clean

# Documentation
docs:
	cargo doc --open

# Load testing targets
load-test-small:
	@echo "Running small scale load test (1,000 nodes)"
	cargo run --release --bin load_test -- --preset small

load-test-medium:
	@echo "Running medium scale load test (50,000 nodes)"
	cargo run --release --bin load_test -- --preset medium

load-test-large:
	@echo "Running large scale load test (500,000 nodes)"
	cargo run --release --bin load_test -- --preset large

load-test-extreme:
	@echo "Running EXTREME boundary test (1M nodes + 5,000 real connections)"
	@echo "WARNING: This pushes system limits - ensure 32GB+ RAM and high file descriptor limits"
	@read -p "This will create 5,000 real network connections (~15GB RAM). Continue? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	cargo run --release --bin load_test -- --preset extreme

load-test-million:
	@echo "Running MILLION NODE load test - This will take significant time and resources!"
	@echo "WARNING: Ensure you have 16GB+ RAM and high file descriptor limits"
	@read -p "Continue? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1
	cargo run --release --bin load_test -- --preset million

# Custom load test
load-test-custom:
	@echo "Running custom load test"
	@echo "Usage: make load-test-custom NODES=10000 DURATION=300 CONNECTIONS=5000"
	cargo run --release --bin load_test -- --nodes $(NODES) --duration $(DURATION) --max-connections $(CONNECTIONS)

# System preparation for load testing
prepare-load-test:
	@echo "Preparing system for load testing..."
	@echo "Current file descriptor limit: $$(ulimit -n)"
	@echo "Recommended: ulimit -n 65536"
	@echo "Current memory: $$(free -h 2>/dev/null || vm_stat | head -5)"
	@echo "Run 'ulimit -n 65536' if needed for high connection counts"

# Run all test suites
test-all: test load-test-small
	@echo "✓ All tests completed successfully"

# Performance benchmarking  
benchmark:
	cargo bench

# Quick validation test
validate:
	@echo "Running quick validation..."
	cargo test --release
	cargo run --release --bin load_test -- --nodes 100 --duration 10
	@echo "✓ SomaSync validation complete"