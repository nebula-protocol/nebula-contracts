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

export async function create_cluster(name: string) {}
