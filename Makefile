all: fmt clippy test build schema

.PHONY: fmt
fmt:
	@echo "Formatting code..."
	@cargo fmt --all -- --check

.PHONY: clippy
clippy:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings

.PHONY: test
test: fmt
	@echo "Running tests..."
	@cargo test

.PHONY: build
build: fmt
	@echo "Building..."
	@cargo build

.PHONY: schema
schema:
	@echo "Generating schema..."
	cd scripts && ./schema.sh

.PHONY: build-optimized
build-optimized: test
	@echo "Building optimized..."
	docker run --rm -v "$(CURDIR)":/code \
		--mount type=volume,source="$(notdir $(CURDIR))_cache",target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/optimizer:0.15.0