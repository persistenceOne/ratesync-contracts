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
    "pool_id": 356,
    "default_bond_denom": "ibc/47847622DF5F34239E71986A819598F8F3727FE2DA021D99058DC31A2C5F364E",
    "stk_token_denom": "ibc/9FF2B7A5F55038A7EE61F4FD6749D9A648B48E89830F2682B67B5DC158E2753C",
    "asset_ordering": "native_token_first"
  }
}
EOF
)

echo "Adding pool to contract..."

echo ">>> $OSMOSISD tx wasm execute $rs_contract_address $msg"
tx=$($OSMOSISD tx wasm execute $rs_contract_address "$msg" --from test $GAS -y -o json)
echo $tx
tx_hash=$(echo $tx | jq -r .txhash)

echo "Tx Hash: $tx_hash"
echo $tx_hash > $METADATA/add_pool_tx_hash_$CONTRACT2.txt
