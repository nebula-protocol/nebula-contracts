import "dotenv/config";
import {
  newClient,
  writeArtifact,
  readArtifact,
  deployContract,
  executeContract,
  uploadContract,
  instantiateContract,
} from "./lib/helpers.js";
import { join } from "path";
import { LCDClient } from "@terra-money/terra.js";

import { uploadAndInit } from "./lib/tx.js";

const ARTIFACTS_PATH = "../artifacts";

// Main
async function main() {}

main().catch(console.log);
