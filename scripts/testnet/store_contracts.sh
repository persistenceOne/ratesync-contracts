#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

function store_contract() {
    echo "Storing contract..."

    CONTRACT_PATH=./artifacts/$1.wasm

    echo ">>> $OSMOSISD tx wasm store $CONTRACT_PATH $GAS"
    tx=$($OSMOSISD tx wasm store $CONTRACT_PATH $GAS --from test -y -o json)
    echo $tx
    tx_hash=$(echo $tx | jq -r .txhash)

    echo "Tx Hash: $tx_hash"
    echo $tx_hash > $METADATA/store_tx_hash_$1.txt

    sleep 10

    code_id=$($OSMOSISD q tx "$tx_hash" -o json | jq -r '.logs[0].events[-1].attributes[-1].value')
    echo "Code ID $1: $code_id"
    echo $code_id > $METADATA/code_id_$1.txt
}

store_contract $CONTRACT1
store_contract $CONTRACT2