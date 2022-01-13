import "dotenv/config";
import {
  newClient,
  writeArtifact,
  readArtifact,
  uploadContract,
} from "../lib/helpers.js";
import { join } from "path";
import { LCDClient } from "@terra-money/terra.js";

import { uploadAndInit } from "../lib/tx.js";

const ARTIFACTS_PATH = "../artifacts";

async function main() {
  console.log("===DEPLOY_PERIPHERAL_START===");
  const { terra, wallet } = newClient();
  console.log(
    `chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`
  );
  let network = readArtifact(terra.config.chainID);

  // Upload cluster and penalty code
  network = await uploadClusterTokenCode(terra, wallet);
  network = await uploadPenaltyCode(terra, wallet);

  // Upload and instantiate peripheral contract
  network = await uploadAndInit("gov", terra, wallet, {
    nebula_token: network.nebTokenAddress,
    quorum: network.gov.quorum,
    threshold: network.gov.threshold,
    voting_period: network.gov.votingPeriod,
    effective_delay: network.gov.effectiveDelay,
    proposal_deposit: network.gov.proposalDeposit,
    voter_weight: network.gov.voterWeight,
    snapshot_period: network.gov.snapshotPeriod,
  });
  network = await uploadAndInit("community", terra, wallet, {
    nebula_token: network.nebTokenAddress,
    owner: network.govAddress,
    spend_limit: network.community.spendLimit,
  });
  network = await uploadAndInit("incentives_custody", terra, wallet, {
    owner: network.govAddress,
    neb_token: network.nebTokenAddress,
  });
  network = await uploadAndInit("oracle", terra, wallet, {
    owner: network.feederAddress,
  });

  writeArtifact(network, terra.config.chainID);

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

async function uploadPenaltyCode(terra: LCDClient, wallet: any) {
  let network = readArtifact(terra.config.chainID);

  console.log("Uploading Penalty code...");

  let resp = await uploadContract(
    terra,
    wallet,
    join(ARTIFACTS_PATH, "nebula_penalty.wasm")
  );

  network["penaltyCodeID"] = resp;
  console.log(`Penalty Code ID: ${network["penaltyCodeID"]}`);
  writeArtifact(network, terra.config.chainID);
  return network;
}

main().catch(console.log);
