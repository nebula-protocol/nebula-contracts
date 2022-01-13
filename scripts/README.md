# Nebula Protocol Scripts

## Overview

### Deploy

This script main handles the deployment of the various smart contracts that makes up the Nebula Protocol. These can be broken down into 4 main categories, each with its own deploy file:

#### [deploy_create_neb](/scripts/deploy/1_deploy_create_neb.ts)

This script sets up environment related to the Nebula governance token.

- Deploys and instantiates the NEB CW20 token using [`astroport_token.wasm`](../artifacts/astro-port_token.wasm)
  - sends the full total supply to the deployer account
- Creates a NEB-UST pair on the Astroport AMM
- Provide initial NEB-UST liquidity on the created pair

### [deploy_peripheral](/scripts/deploy/2_deploy_peripheral.ts)

This handles the uploading and deployment of the various surrounding Nebula contracts. These contracts includes:

- Uploading the [`cluster`](../contracts/nebula-cluster) token contract code
- Uploading the (default) [`penalty`](../contracts/nebula-penalty) contract code
- Deploying the [`gov`](../contracts/nebula-gov) contract
- Deploying the [`community`](../contracts/nebula-community) contract
- Deploying the [`incentives-custody`](../contracts/nebula-incentives-custody) contract
- Deploying the [`oracle`](../contracts/nebula-oracle) contract

### [deploy_core](/scripts/deploy/3_deploy_core.ts)

Deploys and instantiate the following 'main' Nebula contracts.

- [`cluster-factory`](../contracts/nebula-cluster-factory)
- [`lp-staking`](../contracts/nebula-lp-staking)
- [`collector`](../contracts/nebula-collector)
- [`incentives`](../contracts/nebula-incentives)

### [deploy_post_initialize](/scripts/deploy/4_deploy_post_initialize.ts)

This calls `PostInitialize` message on the `cluster-factory` contract

### Execute

Once the contract is deployed, this script sets up the necessary remaining environment:

- Instantiates the `penalty` contract using the default parameters
- Create a test Terra Ecosystem Index cluster comprising of `uluna` and `uusd` with equal weights

### Compile

At each step, the deployed contract addresses are saved in either:

- [`artifacts/bombay-12.json`](/artifacts/bombay-12.json) for testnet deployments
- [`artifacts/columbus-5.json`](/artifacts/columbus-5.json) for mainnet deployments (not recommended using the script)

These addresses are then compiled into a timestamped CSV file in the [`deployments`](./deployments) folder.

## Scripts

### Deploy on `testnet`

Setup Python and Node environment:

```bash 
# install Node modules
npm install

# Setup Python virtual environment and install required packages
python3 -m venv venv
source ven/bin/activate
pip install -r requirements.txt
```

Build contract:

```bash
bash ../fastbuild.sh
```

Create `.env`:

```bash
WALLET="mnemonic"
LCD_CLIENT_URL=https://bombay-lcd.terra.dev
CHAIN_ID=bombay-12

TOKEN_INITIAL_AMOUNT="1000000000000000"
```

Run the script

```bash
npm run start
```
