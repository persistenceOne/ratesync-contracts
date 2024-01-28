#!/bin/bash

set -eu
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source ${SCRIPT_DIR}/vars.sh

msg=$(cat << EOF
{
  "config": {}
}
EOF
)
query_contract $CONTRACT1 "$msg"