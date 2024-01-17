#!/bin/bash

set -eu

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

METADATA=${SCRIPT_DIR}/metadata
mkdir -p $METADATA

OSMOSISD="osmosisd"

$OSMOSISD config node https://rpc.testnet.osmosis.zone:443
$OSMOSISD config chain-id osmo-test-5
$OSMOSISD config keyring-backend test

GAS="--gas-prices 0.1uosmo --gas auto --gas-adjustment 1.5"

CONTRACT1=liquid_stake_rate
CONTRACT2=osmosis_pool_ratesync

function query_contract() {
    echo "Querying contract..."

    contract_address=$(cat $METADATA/contract_address_$1.txt)

    echo ">>> $OSMOSISD q wasm contract-state smart $contract_address $2"
    $OSMOSISD q wasm contract-state smart $contract_address "$msg"
}