#!/usr/bin/env bash

set -e
set -o pipefail

# shellcheck source=./scripts/config.sh
source ./scripts/config.sh

CHAIN_ID="test-core-1"
NODE="--node https://rpc.testnet2.persistence.one:443 --chain-id $CHAIN_ID"

echo "y" | persistenceCore keys delete relayer0 --keyring-backend test
echo "$TEST1_MNEMONIC" | persistenceCore keys add relayer0 --recover --keyring-backend test

TEST1_KEY=$(persistenceCore keys show -a relayer0 --keyring-backend test)
echo "Balance:"
persistenceCore query bank balances "$TEST1_KEY" $NODE
