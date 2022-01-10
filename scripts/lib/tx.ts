import "dotenv/config";
import {
  writeArtifact,
  readArtifact,
  deployContract,
  sleep,
  executeContract,
} from "./helpers.js";
import { join } from "path";
import { LCDClient, Coins } from "@terra-money/terra.js";

const ARTIFACTS_PATH = "../artifacts";

function formatContractName(contract: String) {
  let contractName = "";

  const parts = contract.split("_");
  for (let part of parts) {
    contractName += part.charAt(0).toUpperCase() + part.slice(1);
  }
  return contractName;
}

export async function uploadAndInit(
  contract: string,
  terra: LCDClient,
  wallet: any,
  initMsg: any
) {
  let network = readArtifact(terra.config.chainID);

  const contractName = formatContractName(contract);

  console.log(`Deploying ${contractName} ...`);

  let resp = await deployContract(
    terra,
    wallet,
    "",
    join(ARTIFACTS_PATH, `nebula_${contract}.wasm`),
    initMsg
  );

  let contractJSONKey =
    contractName.charAt(0).toLowerCase() + contractName.slice(1);
  network[`${contractJSONKey}Address`] = resp.shift();
  console.log(
    `Address ${contractName} Contract: ${network[`${contractJSONKey}Address`]}`
  );
  writeArtifact(network, terra.config.chainID);
  await sleep(10000);
  return network;
}

export async function execute(
  func: string,
  contractAddress: string,
  terra: LCDClient,
  wallet: any,
  msg: any
) {
  let network = readArtifact(terra.config.chainID);

  console.log(`Executing ${func} on ${contractAddress}`);

  let tx = await executeContract(
    terra,
    wallet,
    contractAddress,
    msg,
    new Coins([])
  );

  console.log(`${func} excuted: ${tx.txhash}`);
  await sleep(10000);

  return tx.txhash;
}
