#!/usr/bin/env bash

set -e
set -o pipefail

# See the list of codes that was uploaded to the testnet previously.
# persistenceCore query wasm list-code --node https://rpc.testnet.persistence.one:443
# You can set the node to the persistenceCore config and don't have to worry about passing that flag always
CONFIG_JSON="ptoken.json"

# persistenceCore config node https://rpc.testnet.persistence.one:443
# persistenceCore config node https://rpc.devnet.bamboo.zone:443
# To upload the contract via proposal

CHAIN_ID="test-core-1"
# CHAIN_ID="persistencecore"
NODE="--node https://rpc.testnet2.persistence.one:443"
# NODE="--node https://rpc.devnet.core.dexter.zone:443"

TEST1_MNEMONIC=${TEST1_MNEMONIC:-"cheap enter flush job motion explain deny music clinic harsh uphold payment parade erosion enter regret escape stable remove menu hold village theme favorite"}
# TEST1_MNEMONIC=${TEST1_MNEMONIC:-"exist spirit aspect must pumpkin virtual people shy achieve actual digital scare silly cushion phrase reflect surprise you lonely sail hazard act skate sketch"}
# TEST1_MNEMONIC=${TEST1_MNEMONIC:-"business base spread bottom parade vivid monitor claw raccoon term inject range pyramid train size month rib beauty magic cigar fire orbit amateur olive"}
# TEST1_MNEMONIC=${TEST1_MNEMONIC:-"dentist torch gaze enable ice hospital silver come develop anxiety broken leader language found unfair rent cattle occur fever super fabric cricket toward nerve"}
# TEST1_MNEMONIC=${TEST1_MNEMONIC:-"middle weather hip ghost quick oxygen awful library broken chicken tackle animal crunch appear fee indoor fitness enough orphan trend tackle faint eyebrow all"}
echo "y" | persistenceCore keys delete user1_testnet --keyring-backend test
echo "$TEST1_MNEMONIC" | persistenceCore keys add user1_testnet --recover --keyring-backend test

TEST1_KEY=$(persistenceCore keys show -a user1_testnet --keyring-backend test)
echo "Balance:"
persistenceCore query bank balances "$TEST1_KEY" $NODE