import "dotenv/config";
import { newClient, readArtifact } from "./helpers.js";
import { uploadAndInit } from "./lib.js";

const ARTIFACTS_PATH = "../artifacts";

async function main() {
  console.log("===DEPLOY_CORE_START===");
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

  network = await uploadAndInit("cluster_factory", terra, wallet, {
    token_code_id: network.tokenCodeID,
    cluster_code_id: network.clusterTokenCodeID,
    base_denom: network.baseDenom,
    protocol_fee_rate: "0.001",
    distribution_schedule: network.clusterFactory.distributionSchedule,
  });
  network = await uploadAndInit("lp_staking", terra, wallet, {
    owner: network.clusterFactoryAddress,
    nebula_token: network.tokenAddress,
    terraswap_factory: network.terraswapFactoryAddress,
  });
  network = await uploadAndInit("collector", terra, wallet, {
    distribution_contract: network.govAddress,
    terraswap_factory: network.terraswapFactoryAddress,
    nebula_token: network.tokenAddress,
    base_denom: network.baseDenom,
    owner: network.clusterFactoryAddress,
  });
  network = await uploadAndInit("incentives", terra, wallet, {
    factory: network.clusterFactoryAddress,
    custody: network.incentivesCustodyAddress,
    terraswap_factory: network.terraswapFactoryAddress,
    nebula_token: network.tokenAddress,
    base_denom: network.baseDenom,
    owner: wallet.key.accAddress,
  });

  console.log("===DEPLOY_CORE_FINISH===");
}

main().catch(console.log);
