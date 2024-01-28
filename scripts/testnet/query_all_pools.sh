#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

msg=$(cat << EOF
{
  "all_pools": {}
}
EOF
)
query_contract $CONTRACT2 "$msg"