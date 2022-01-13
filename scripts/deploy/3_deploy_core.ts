import "dotenv/config";
import { newClient, readArtifact, writeArtifact } from "../lib/helpers.js";
import { uploadAndInit } from "../lib/tx.js";

async function main() {
  // Setup
  console.log("===DEPLOY_CORE_START===");
  const { terra, wallet } = newClient();
  console.log(
    `chainID: ${terra.config.chainID} wallet: ${wallet.key.accAddress}`
  );
  let network = readArtifact(terra.config.chainID);

  // Deploy core contracts
  network = await uploadAndInit("cluster_factory", terra, wallet, {
    token_code_id: network.tokenCodeID,
    cluster_code_id: network.clusterTokenCodeID,
    base_denom: network.baseDenom,
    protocol_fee_rate: "0.001",
    distribution_schedule: network.clusterFactory.distributionSchedule,
  });
  network = await uploadAndInit("lp_staking", terra, wallet, {
    owner: network.clusterFactoryAddress,
    nebula_token: network.nebTokenAddress,
    astroport_factory: network.astroportFactoryAddress,
  });
  network = await uploadAndInit("collector", terra, wallet, {
    distribution_contract: network.govAddress,
    astroport_factory: network.astroportFactoryAddress,
    nebula_token: network.nebTokenAddress,
    base_denom: network.baseDenom,
    owner: network.clusterFactoryAddress,
  });
  network = await uploadAndInit("incentives", terra, wallet, {
    factory: network.clusterFactoryAddress,
    custody: network.incentivesCustodyAddress,
    astroport_factory: network.astroportFactoryAddress,
    nebula_token: network.nebTokenAddress,
    base_denom: network.baseDenom,
    owner: wallet.key.accAddress,
  });

  writeArtifact(network, terra.config.chainID);
  console.log("===DEPLOY_CORE_FINISH===");
}

main().catch(console.log);
