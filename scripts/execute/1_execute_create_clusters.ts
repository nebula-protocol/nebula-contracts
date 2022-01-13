import "dotenv/config";
import {
  newClient,
  readArtifact,
  instantiateContract,
  NativeAsset,
} from "../lib/helpers.js";

import { createCluster } from "../lib/clusters.js";

// Main
async function main() {
  // Setup
  console.log("===EXECUTE_POST_INITIALIZE_START===");
  const { terra, wallet } = newClient();
  console.log(
    `chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`
  );
  let network = readArtifact(terra.config.chainID);
  const clusterFactoryAddress = network.clusterFactoryAddress;

  // Instantiate penalty contract
  let instantiateResponse = await instantiateContract(
    terra,
    wallet,
    "",
    network.penaltyCodeID,
    {
      owner: clusterFactoryAddress,
      penalty_params: {
        penalty_amt_lo: "0.02",
        penalty_cutoff_lo: "0.01",
        penalty_amt_hi: "1",
        penalty_cutoff_hi: "0.1",
        reward_amt: "0.01",
        reward_cutoff: "0.02",
      },
    }
  );
  let penaltyAddress = instantiateResponse[0];
  console.log(`instantiated penalty contract: ${penaltyAddress}`);

  // create EOA TER Cluster
  network = await createCluster(
    "TER",
    {
      name: "Terra Ecosystem Index",
      description: "Terra Ecosystem Index Cluster",
      symbol: "TER",
      penalty: penaltyAddress,
      target: [
        new NativeAsset("uusd", "1000000000").withAmount(),
        new NativeAsset("uluna", "1000000000").withAmount(),
      ],
      pricing_oracle: network.oracleAddress,
      target_oracle: network.targetAddress,
    },
    terra,
    wallet
  );
}

main().catch(console.log);
