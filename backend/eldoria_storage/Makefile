.PHONY: test test-unit test-integration test-property test-all clean coverage

test: test-unit

test-unit:
	cargo test --lib --features test-utils

test-integration:
	cargo test --test integration --features test-utils

test-property:
	cargo test --test property_based --features test-utils

test-all:
	cargo test --all-features

test-verbose:
	cargo test --all-features -- --nocapture

coverage:
	cargo tarpaulin --all-features --out Html --output-dir coverage

clean:
	cargo clean
	rm -rf coverage/

# Run tests with specific backend features
test-postgres:
	cargo test --features postgres,test-utils

test-redis:
	cargo test --features redis,test-utils

test-surrealdb:
	cargo test --features surrealdb,test-utils

# Run tests in CI mode (all features, verbose)
ci:
	cargo test --all-features -- --test-threads=1 --nocapture
