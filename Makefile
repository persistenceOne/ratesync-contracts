.PHONY: fmt
fmt:
	@echo "Formatting code..."
	@cargo fmt --all -- --check

.PHONY: test
test: fmt
	@echo "Running tests..."
	@cargo test

.PHONY: build
build: fmt
	@echo "Building..."
	@cargo build

.PHONY: build-optimize
build-optimize: test
	@docker run --rm -v "$(pwd)":/code \
		--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer:0.14.0