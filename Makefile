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

################# testnet #################
.PHONY: store-contracts-testnet
store-contracts-testnet:
	@echo "Deploying to testnet..."
	@bash scripts/testnet/store_contracts.sh

.PHONY: query-config
query-config:
	@echo "Querying config..."
	@bash scripts/testnet/query_config.sh

.PHONY: query-historical
query-historical:
	@echo "Querying historical..."
	@bash scripts/testnet/query_historical.sh

.PHONY: query-latest
query-latest:
	@echo "Querying latest..."
	@bash scripts/testnet/query_latest.sh

.PHONY: instantiate-contract
instantiate-contract:
	@echo "Instantiating contract..."
	@bash scripts/testnet/instantiate_contract.sh
