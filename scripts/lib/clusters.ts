import "dotenv/config";
import {
  readArtifact,
  executeContract,
  queryContract,
  writeArtifact,
} from "./helpers.js";
import { LCDClient } from "@terra-money/terra.js";

export async function createCluster(
  name: string,
  params: any,
  terra: LCDClient,
  wallet: any
) {
  let network = readArtifact(terra.config.chainID);
  const clusterFactoryAddress = network.clusterFactoryAddress;

  // Create clusters
  console.log(`Creating ${name} Cluster`);
  console.log(`Executing create_cluster on ${clusterFactoryAddress}`);
  let createClusterTx = await executeContract(
    terra,
    wallet,
    clusterFactoryAddress,
    {
      create_cluster: {
        params,
      },
    }
  );
  const createClusterHash = createClusterTx.txhash;
  console.log(`create_cluster excuted: ${createClusterHash}`);
  const clusterContractAddress = createClusterTx.logs[0].events[1].attributes
    .filter((element) => element.key == "cluster_addr")
    .map((x) => x.value)[0];
  console.log(`Cluster contract address: ${clusterContractAddress}`);

  const clusterConfig = await queryContract(terra, clusterContractAddress, {
    config: {},
  });
  const clusterTokenAddress = clusterConfig.config.cluster_token;
  console.log(`Cluster token address: ${clusterTokenAddress}`);

  network[`${name.toLowerCase()}ContractAddress`] = clusterContractAddress;
  network[`${name.toLowerCase()}TokenAddress`] = clusterTokenAddress;
  network[`${name.toLowerCase()}PenaltyAddress`] = params.penalty;

  writeArtifact(network, terra.config.chainID);
  return network;
}
