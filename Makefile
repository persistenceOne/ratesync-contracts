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

.PHONY: build-optimized
build-optimized: test
	@echo "Building optimized..."
	docker run --rm -v "$(CURDIR)":/code \
		--mount type=volume,source="$(notdir $(CURDIR))_cache",target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer:0.14.0