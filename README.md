# Persistence-Osmosis ICA

## Design

The `redemption-rate` contract is a simple contract that allows for the calculation of the redemption rate of a given pool. The redemption rate is the ratio of the value of the underlying assets to the value of the pool tokens.

This contract is used by the `ica` to store the redemption rate of the pool on host chain.

The rate is sent from controller chain to host chain via IBC. The `ica` contract then uses the rate to be stored in this `redemption-rate` contract.

## Rust Contracts

The concrete logic is stored in `contracts`.

To ensure they are proper, run the following in the repo root:

```shell
cargo build --all-targets
cargo clippy --all-targets
cargo fmt
```

## Unit Tests

All unit tests are in Rust and assume a mocked out environment.

To ensure they are proper, run the following in the repo root:

```shell
cargo test
```

## Integration Tests

See [tests/README.md](tests/README.md)
