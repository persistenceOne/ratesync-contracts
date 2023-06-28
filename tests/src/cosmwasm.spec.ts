import { CosmWasmSigner, testutils } from "@confio/relayer";
import { assert } from "@cosmjs/utils";
import test from "ava";

import {
  InstantiateMsg,
  Price,
  RedemptionRateResponse,
} from "./bindings/RedemptionRate.types";
import config from "./config";
import { setupContracts, setupOsmosisClient, setupWasmClient } from "./utils";

let osmosisIds: Record<string, number> = {};

test.before(async (t) => {
  console.debug("Upload contracts to osmosis...");
  const osmosisContracts = {
    redemptionRate: "./internal/redemption_rate.wasm",
  };
  const osmosisSign = await setupOsmosisClient();
  osmosisIds = await setupContracts(osmosisSign, osmosisContracts);

  t.pass();
});

test.serial("deploy contract", async (t) => {
  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initRedemptionRateMsg: InstantiateMsg = {
    config: {
      owner: config.IcaAddress,
    },
  };
  const { contractAddress: osmoRedemptionRateContract } =
    await osmoClient.sign.instantiate(
      osmoClient.senderAddress,
      osmosisIds.redemptionRate,
      initRedemptionRateMsg,
      "redemption rate",
      "auto"
    );
  t.truthy(osmoRedemptionRateContract);
  t.log(`Contract: ${osmoRedemptionRateContract}`);
});

interface SetupInfo {
  wasmClient: CosmWasmSigner;
  osmoClient: CosmWasmSigner;
  osmoRedemptionRateContract: string;
}

async function demoSetup(): Promise<SetupInfo> {
  // instantiate ica controller on wasmd
  const wasmClient = await setupWasmClient();

  // instantiate ica host on osmosis
  const osmoClient = await setupOsmosisClient();
  const initRedemptionRateMsg: InstantiateMsg = {
    config: {
      owner: config.HostWallet,
    },
  };
  const { contractAddress: osmoRedemptionRateContract } =
    await osmoClient.sign.instantiate(
      osmoClient.senderAddress,
      osmosisIds.redemptionRate,
      initRedemptionRateMsg,
      "redemption rate",
      "auto"
    );
  assert(osmoRedemptionRateContract);

  return {
    wasmClient,
    osmoClient,
    osmoRedemptionRateContract,
  };
}

test.serial("Set and query redemption rate", async (t) => {
  const { wasmClient, osmoClient, osmoRedemptionRateContract } =
    await demoSetup();

  const price: Price = {
    denom: "atom",
    base_denom: "stkAtom",
  };

  const setRedemptionRateMsg = {
    set_redemption_rate: {
      exchange_rate: "1.2",
      price,
    },
  };
  const result = await osmoClient.sign.execute(
    osmoClient.senderAddress,
    osmoRedemptionRateContract,
    setRedemptionRateMsg,
    "auto"
  );
  t.truthy(result);
  t.log(`Result: ${JSON.stringify(result)}`);

  // query the redemption rate
  const queryRedemptionRateMsg = {
    redemption_rate_request: {
      price,
    },
  };

  const queryResult: RedemptionRateResponse =
    await osmoClient.sign.queryContractSmart(
      osmoRedemptionRateContract,
      queryRedemptionRateMsg
    );
  t.truthy(queryResult);
  t.log(`Query result: ${JSON.stringify(queryResult)}`);
});
