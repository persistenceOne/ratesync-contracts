#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

owner_address=$($OSMOSISD keys show test -a)
lsr_contract_address=$(cat $METADATA/contract_address_$CONTRACT1.txt)

code_id=$(cat $METADATA/code_id_$CONTRACT2.txt)
init_msg=$(cat << EOF
{
  "owner_address": "$owner_address",
  "lsr_contract_address": "$lsr_contract_address"
}
EOF
)

echo "Instantiating contract..."

echo ">>> $OSMOSISD tx wasm instantiate $code_id $init_msg"
tx=$($OSMOSISD tx wasm instantiate $code_id "$init_msg" --from test --label "persistence_osmosis_ratesync" --no-admin $GAS -y -o json)
echo $tx
tx_hash=$(echo $tx | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo "instantiate $CONTRACT2: $tx_hash" >> $METADATA/tx_logs.txt

sleep 10

contract_address=$($OSMOSISD q wasm list-contract-by-code "$code_id" -o json | jq -r '.contracts[-1]')
echo "Contract Address: $contract_address"
echo $contract_address > $METADATA/contract_address_$CONTRACT2.txt
