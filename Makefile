all: start-chains rly-wallets rly-init

.PHONY: start-chains
start-chains:
	@echo "Starting chains..."
	./scripts/wasmd/start.sh
	./scripts/osmosis/start.sh
	sleep 5

.PHONY: stop-chains
stop-chains:
	@echo "Stopping chains..."
	./scripts/osmosis/stop.sh || true
	./scripts/wasmd/stop.sh || true

.PHONY: clean
clean: stop-chains
	@echo "Cleaning..."
	rm -rf data

.PHONY: rly-wallets
rly-wallets:
	@echo "Initializing wallets..."
	./network/init.sh

.PHONY: rly-init
rly-init:
	@echo "Initializing relayer..."
	./network/relayer/interchain-acc-config/rly-init.sh
	make rly-ls
	make rly-paths

.PHONY: rly-start
rly-start:
	@echo "Starting relayer..."
	./network/relayer/interchain-acc-config/rly-start.sh

.PHONY: rly-ls
rly-ls:
	@echo "Listing relayer chains..."
	rly chains list --home data/relayer

.PHONY: rly-paths
rly-paths:
	@echo "Relayer paths list..."
	rly paths list --home data/relayer
