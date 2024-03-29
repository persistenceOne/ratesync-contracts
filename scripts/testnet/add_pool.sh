#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

owner_address=$($OSMOSISD keys show test -a)
rs_contract_address=$(cat $METADATA/contract_address_$CONTRACT2.txt)

code_id=$(cat $METADATA/code_id_$CONTRACT2.txt)
msg=$(cat << EOF
{
  "add_pool": {
    "pool_id": 374,
    "stk_token_denom": "stk/uatom",
    "transfer_port_id": "transfer",
    "transfer_channel_id": "channel-4198",
    "asset_ordering": "stk_token_first"
  }
}
EOF
)

echo "Adding pool to contract..."

echo ">>> $OSMOSISD tx wasm execute $rs_contract_address $msg"
tx=$($OSMOSISD tx wasm execute $rs_contract_address "$msg" --from test $GAS -y -o json)
tx_hash=$(echo $tx | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo "add_pool: $tx_hash" >> $METADATA/tx_logs.txt
