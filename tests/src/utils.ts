import { readFileSync } from "fs";

import { CosmWasmSigner, testutils } from "@confio/relayer";

import config from "./config";

const {
  fundAccount,
  generateMnemonic,
  osmosis: oldOsmo,
  signingCosmWasmClient,
  wasmd: oldWasmd,
} = testutils;

const wasmd = {
  ...oldWasmd,
  prefix: "stride",
  tendermintUrlHttp: "http://localhost:26657",
  chainId: "STRIDE",
  denomFee: "ustrd",
  minFee: "0.025ustrd",
};

const osmosis = {
  ...oldOsmo,
  tendermintUrlHttp: "http://localhost:26357",
  chainId: "OSMO",
  minFee: "0.025uosmo",
};

export async function setupContracts(
  cosmwasm: CosmWasmSigner,
  contracts: Record<string, string>
): Promise<Record<string, number>> {
  const results: Record<string, number> = {};

  for (const name in contracts) {
    const path = contracts[name];
    console.info(`Storing ${name} from ${path}...`);
    const wasm = await readFileSync(path);
    const receipt = await cosmwasm.sign.upload(
      cosmwasm.senderAddress,
      wasm,
      "auto",
      `Upload ${name}`
    );
    console.debug(`Upload ${name} with CodeID: ${receipt.codeId}`);
    results[name] = receipt.codeId;
  }

  return results;
}

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupWasmClient(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  const mnemonic = config.WalletMnemonic;
  const cosmwasm = await signingCosmWasmClient(wasmd, mnemonic);
  return cosmwasm;
}

// This creates a client for the CosmWasm chain, that can interact with contracts
export async function setupOsmosisClient(): Promise<CosmWasmSigner> {
  // create apps and fund an account
  const mnemonic = config.WalletMnemonic;
  const cosmwasm = await signingCosmWasmClient(osmosis, mnemonic);
  return cosmwasm;
}
