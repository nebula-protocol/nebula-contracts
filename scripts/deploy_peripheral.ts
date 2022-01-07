import "dotenv/config";
import {
  newClient,
  writeArtifact,
  readArtifact,
  deployContract,
  executeContract,
  uploadContract,
  instantiateContract,
} from "./helpers.js";
import { join } from "path";
import { LCDClient } from "@terra-money/terra.js";

import { uploadAndInit } from "./lib.js";

const ARTIFACTS_PATH = "../artifacts";

async function main() {
  console.log("===DEPLOY_PERIPHERAL_START===");
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

  network = await uploadClusterTokenCode(terra, wallet);

  network = await uploadAndInit("gov", terra, wallet, {
    nebula_token: network.tokenAddress,
    quorum: network.gov.quorum,
    threshold: network.gov.threshold,
    voting_period: network.gov.votingPeriod,
    effective_delay: network.gov.effectiveDelay,
    proposal_deposit: network.gov.proposalDeposit,
    voter_weight: network.gov.voterWeight,
    snapshot_period: network.gov.snapshotPeriod,
  });
  network = await uploadAndInit("community", terra, wallet, {
    nebula_token: network.tokenAddress,
    owner: network.govAddress,
    spend_limit: network.community.spendLimit,
  });
  network = await uploadAndInit("incentives_custody", terra, wallet, {
    owner: network.govAddress,
    neb_token: network.tokenAddress,
  });
  network = await uploadAndInit("oracle", terra, wallet, {
    owner: network.feederAddress,
  });

  // Set new owner for admin
  network = readArtifact(terra.config.chainID); // reload variables

  console.log("===DEPLOY_PERIPHERAL_FINISH===");
}

async function uploadClusterTokenCode(terra: LCDClient, wallet: any) {
  let network = readArtifact(terra.config.chainID);

  console.log("Uploading Cluster code...");

  let resp = await uploadContract(
    terra,
    wallet,
    join(ARTIFACTS_PATH, "nebula_cluster.wasm")
  );

  network["clusterTokenCodeID"] = resp;
  console.log(`Cluster Token Code ID: ${network["clusterTokenCodeID"]}`);
  writeArtifact(network, terra.config.chainID);
  return network;
}

main().catch(console.log);
