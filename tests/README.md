# test-redemption-rate

## Setup

Ensure you have node 14+ (16+ recommended):

```
node --version
```

Then install via npm as typical:

```
npm install
```

## Development

Build the source:

```
npm run build
```

Clean it up with prettier and eslint:

```
npm run fix
```

## Testing

### Build optimized contract WASM

Compile the contracts for uploading.

```sh
./scripts/build_integration_wasm.sh
```

NOTE: you need to run this each time your contract changes.

### Run two chains in docker

This actually runs the test codes on contracts. To do so, we need to start two blockchains in the background and then run the process. This may require that you have docker installed and running on your local machine. If you don't, please do that first before running the scripts.

### Run tests

Terminal 3:

```
npm run test
```

You may run and re-run tests many times. When you are done with it and want to free up some system resources (stop running two blockchains in the background), you need to run these commands to stop them properly:

## Codegen

We use [ts-codegen](https://github.com/CosmWasm/ts-codegen) to generate bindings to some contracts.
Read for more info of follow this quick start:

Install ts-codegen

```bash
npm install -g @cosmwasm/ts-codegen
```

Generate schema for the contract

```bash
cd ls ../contracts/redemption-rate
cargo schema
cd -
```

Generate bindings

```bash
mkdir -p src/bindings

cosmwasm-ts-codegen generate \
          --plugin client \
          --schema ls ../contracts/redemption-rate/schema \
          --out ./src/bindings \
          --name RedemptionRate
```

(You can safely say "no" for "enable bundle")
