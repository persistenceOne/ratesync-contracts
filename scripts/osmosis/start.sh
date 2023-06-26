#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

# Please keep this in sync with the Ports overview in HACKING.md
TENDERMINT_PORT_GUEST="26657"
TENDERMINT_PORT_HOST="26653"
GRPC_PORT_GUEST="9090"
GRPC_PORT_HOST="9086"
GRPC_WEB_PORT_GUEST="9091"
GRPC_WEB_PORT_HOST="9087"

SCRIPT_DIR="$(realpath "$(dirname "$0")")"
# shellcheck source=./env
# shellcheck disable=SC1091
source "$SCRIPT_DIR"/env

# Use a fresh volume for every start
docker volume rm -f osmosis_data
# only pull if we don't have it
(docker images | grep "$REPOSITORY" | grep -q "$VERSION") || docker pull "$REPOSITORY:$VERSION"

echo "starting osmosisd running on http://localhost:$TENDERMINT_PORT_HOST"

docker run -d --rm \
  --user=root \
  --name "$CONTAINER_NAME" \
  -p "$TENDERMINT_PORT_HOST":"$TENDERMINT_PORT_GUEST" \
  -p "$GRPC_PORT_HOST":"$GRPC_PORT_GUEST" \
  -p "$GRPC_WEB_PORT_HOST":"$GRPC_WEB_PORT_GUEST" \
  --mount type=bind,source="$SCRIPT_DIR/template",target=/template \
  --mount type=volume,source=osmosis_data,target=/root \
  "$REPOSITORY:$VERSION"
