#!/bin/bash

# Configure predefined mnemonic pharses
BINARY=rly
CHAIN_DIR=./data
CHAINID_1=wasmd-1
CHAINID_2=osmo-testing
PATH_NAME=wasm-osmo
RELAYER_DIR=./relayer
MNEMONIC_1="alley afraid soup fall idea toss can goose become valve initial strong forward bright dish figure check leopard decide warfare hub unusual join cart"
MNEMONIC_2="record gift you once hip style during joke field prize dust unique length more pencil transfer quit train device arrive energy sort steak upset"

# Ensure rly is installed
if ! [ -x "$(command -v $BINARY)" ]; then
    echo "$BINARY is required to run this script..."
    echo "You can download at https://github.com/cosmos/relayer"
    exit 1
fi

echo "Starting to listen relayer..."
$BINARY start $PATH_NAME -p events -b 100 --home $CHAIN_DIR/$RELAYER_DIR
