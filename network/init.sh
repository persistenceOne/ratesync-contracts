#!/bin/bash

BINARY_1=wasmd
BINARY_2=osmosisd
CHAIN_DIR=./data
CHAINID_1=wasmd-1
CHAINID_2=osmo-testing

VAL_MNEMONIC_1="enlist hip relief stomach skate base shallow young switch frequent cry park"
VAL_MNEMONIC_2="remain fragile remove stamp quiz bus country dress critic mammal office need"
RLY_MNEMONIC_1="alley afraid soup fall idea toss can goose become valve initial strong forward bright dish figure check leopard decide warfare hub unusual join cart"
RLY_MNEMONIC_2="record gift you once hip style during joke field prize dust unique length more pencil transfer quit train device arrive energy sort steak upset"


P2PPORT_1=16656
P2PPORT_2=26656
RPCPORT_1=26659
RPCPORT_2=26653
RESTPORT_1=1316
RESTPORT_2=1317
ROSETTA_1=8080
ROSETTA_2=8081

echo "Removing previous data..."
rm -rf $CHAIN_DIR/$CHAINID_1 &> /dev/null
rm -rf $CHAIN_DIR/$CHAINID_2 &> /dev/null

# Add directories for both chains, exit if an error occurs
if ! mkdir -p $CHAIN_DIR/$CHAINID_1 2>/dev/null; then
    echo "Failed to create chain folder. Aborting..."
    exit 1
fi

if ! mkdir -p $CHAIN_DIR/$CHAINID_2 2>/dev/null; then
    echo "Failed to create chain folder. Aborting..."
    exit 1
fi

echo "Adding relayer accounts..."
echo $VAL_MNEMONIC_1 | $BINARY_1 keys add validator --home $CHAIN_DIR/$CHAINID_1 --recover --keyring-backend=test
echo $RLY_MNEMONIC_1 | $BINARY_1 keys add rly1 --home $CHAIN_DIR/$CHAINID_1 --recover --keyring-backend=test 
echo $RLY_MNEMONIC_2 | $BINARY_1 keys add rly2 --home $CHAIN_DIR/$CHAINID_1 --recover --keyring-backend=test 

echo $VAL_MNEMONIC_2 | $BINARY_2 keys add validator --home $CHAIN_DIR/$CHAINID_2 --recover --keyring-backend=test
echo $RLY_MNEMONIC_1 | $BINARY_2 keys add rly1 --home $CHAIN_DIR/$CHAINID_2 --recover --keyring-backend=test 
echo $RLY_MNEMONIC_2 | $BINARY_2 keys add rly2 --home $CHAIN_DIR/$CHAINID_2 --recover --keyring-backend=test

echo "Configuring chain binaries..."
$BINARY_1 config node http://localhost:$RPCPORT_1 --home $CHAIN_DIR/$CHAINID_1
$BINARY_2 config node http://localhost:$RPCPORT_2 --home $CHAIN_DIR/$CHAINID_2

echo "Adding funds..."
VAL1_KEY=$($BINARY_1 keys show -a validator --home $CHAIN_DIR/$CHAINID_1 --keyring-backend test)
CHAIN1_RLY1_KEY=$($BINARY_1 keys show -a rly1 --home $CHAIN_DIR/$CHAINID_1 --keyring-backend test)
CHAIN1_RLY2_KEY=$($BINARY_1 keys show -a rly2 --home $CHAIN_DIR/$CHAINID_1 --keyring-backend test)

VAL2_KEY=$($BINARY_2 keys show -a validator --home $CHAIN_DIR/$CHAINID_2 --keyring-backend test)
CHAIN2_RLY1_KEY=$($BINARY_2 keys show -a rly1 --home $CHAIN_DIR/$CHAINID_2 --keyring-backend test)
CHAIN2_RLY2_KEY=$($BINARY_2 keys show -a rly2 --home $CHAIN_DIR/$CHAINID_2 --keyring-backend test)

echo "Balance:"
$BINARY_1 query bank balances "$VAL1_KEY" --home $CHAIN_DIR/$CHAINID_1
$BINARY_2 query bank balances "$VAL2_KEY" --home $CHAIN_DIR/$CHAINID_2

echo "Sending tokens..."
$BINARY_1 tx bank send "$VAL1_KEY" "$CHAIN1_RLY1_KEY" 1000000ucosm --home $CHAIN_DIR/$CHAINID_1 --keyring-backend test --yes --chain-id $CHAINID_1 --fees 5000ucosm
sleep 5
$BINARY_1 tx bank send "$VAL1_KEY" "$CHAIN1_RLY2_KEY" 1000000ucosm --home $CHAIN_DIR/$CHAINID_1 --keyring-backend test --yes --chain-id $CHAINID_1 --fees 5000ucosm
sleep 5
$BINARY_2 tx bank send "$VAL2_KEY" "$CHAIN2_RLY1_KEY" 1000000uosmo --home $CHAIN_DIR/$CHAINID_2 --keyring-backend test --yes --chain-id $CHAINID_2
sleep 5
$BINARY_2 tx bank send "$VAL2_KEY" "$CHAIN2_RLY2_KEY" 1000000uosmo --home $CHAIN_DIR/$CHAINID_2 --keyring-backend test --yes --chain-id $CHAINID_2
