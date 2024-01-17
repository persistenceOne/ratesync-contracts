#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

msg=$(cat << EOF
{
  "historical_liquid_stake_rates": {
    "default_bond_denom": "uatom",
    "stk_denom": "stk/uatom",
    "limit": 10
  }
}
EOF
)
query_contract $CONTRACT1 "$msg"