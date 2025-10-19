# SomaSync Testing Guide

## Quick Start

For CI/development, run fast tests only:
```bash
make test
```

## Test Categories

### 1. Fast Tests (CI-friendly)
- **Unit Tests**: 23 core library tests
- **Integration Tests**: 7 real-world scenario tests
- **Runtime**: ~1 second total
- **Command**: `make test`

### 2. Load Tests (Manual only)
- **Small Scale**: 10K nodes (~30s)
- **Medium Scale**: 100K nodes (~60s) 
- **Large Scale**: 500K+ nodes (several minutes)
- **Command**: `make test-with-load` or individual `make load-test-*`

## Available Commands

```bash
# Fast development cycle (recommended for CI)
make test                    # Unit + integration tests only

# Manual performance validation
make test-with-load         # Include load test suite
make load-test-small        # Quick 1K node validation
make load-test-medium       # 50K node test
make load-test-large        # 500K node test
make load-test-extreme      # 1M nodes + 5K real connections
make load-test-million      # Full million node simulation

# Other targets
make test-all               # Info about test options
make build                  # Build only
make release               # Release build
```

## CI Configuration

The default `make test` target is optimized for CI:
- Excludes resource-intensive load tests
- Runs in ~1 second
- Validates core functionality and integration
- Zero false positives from system resource limits

Load tests are gated behind the `load-tests` feature flag and are disabled by default.

## Development Workflow

1. **Regular development**: Use `make test` for fast feedback
2. **Performance validation**: Run `make load-test-small` occasionally  
3. **Pre-release**: Run full `make test-with-load` suite
4. **Stress testing**: Use individual load test targets as needed

## Test Structure

- `src/lib.rs` - Unit tests for all modules
- `tests/integration_mesh_networking.rs` - Integration scenarios
- `src/bin/load_test.rs` - Performance testing binary
- `src/bin/load_test_million_nodes.rs` - Extreme scale testing

Load test binaries are excluded from default test runs but available for manual performance validation.