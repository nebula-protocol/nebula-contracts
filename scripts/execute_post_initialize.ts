import { Coins } from "@terra-money/terra.js";
import "dotenv/config";
import {
  executeContract,
  NativeAsset,
  newClient,
  readArtifact,
  TokenAsset,
} from "./lib/helpers.js";
import { uploadAndInit, execute } from "./lib/tx";

const ARTIFACTS_PATH = "../artifacts";

async function main() {
  console.log("===EXECUTE_POST_INITIALIZE_START===");
  const { terra, wallet } = newClient();
  console.log(
    `chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`
  );
  let network = readArtifact(terra.config.chainID);

  if (!network.tokenAddress) {
    console.log(
      `Please deploy the CW20-base ASTRO token, and then set this address in the deploy config before running this script...`
    );
    return;
  }

  let nebUSTAssetInfos = [
    new TokenAsset(network.tokenAddress).getInfo(),
    new NativeAsset("uusd").getInfo(),
  ];

  let nebUSTPairCreationTx = await execute(
    "create_pair",
    network.astroportFactoryAddress,
    terra,
    wallet,
    {
      create_pair: {
        asset_infos: nebUSTAssetInfos,
      },
    }
  );

  let postInitializeTx = await execute(
    "post_initialize",
    network.clusterFactoryAddress,
    terra,
    wallet,
    {
      post_initialize: {
        owner: wallet.key.accAddress,
        astroport_factory: network.astroportFactoryAddress,
        nebula_token: network.tokenAddress,
        staking_contract: network.lpStakingAddress,
        commission_collector: network.collectorAddress,
      },
    }
  );

  console.log("===EXECUTE_POST_INITIALIZE_FINISH===");
}

main().catch(console.log);
