#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

msg=$(cat << EOF
{
  "liquid_stake_rate": {
    "default_bond_denom": "uatom",
    "stk_denom": "stk/uatom"
  }
}
EOF
)
query_contract $CONTRACT1 "$msg"