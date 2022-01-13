import "dotenv/config";
import { newClient, readArtifact, writeArtifact } from "../lib/helpers.js";
import { execute } from "../lib/tx.js";

const ARTIFACTS_PATH = "../artifacts";

async function main() {
  console.log("===EXECUTE_POST_INITIALIZE_START===");
  const { terra, wallet } = newClient();
  console.log(
    `chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`
  );
  let network = readArtifact(terra.config.chainID);

  // Post-initialize Nebula Factory contract
  await execute(
    "post_initialize",
    network.clusterFactoryAddress,
    terra,
    wallet,
    {
      post_initialize: {
        owner: wallet.key.accAddress,
        astroport_factory: network.astroportFactoryAddress,
        nebula_token: network.nebTokenAddress,
        staking_contract: network.lpStakingAddress,
        commission_collector: network.collectorAddress,
      },
    }
  );

  writeArtifact(network, terra.config.chainID);

  console.log("===EXECUTE_POST_INITIALIZE_FINISH===");
}

main().catch(console.log);
