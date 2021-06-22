# Nebula Protocol Contracts

This repository contains the full source code for Nebula Protocol on Terra.

## Contracts

### Nebula Core

These contracts hold the core logic of the base protocol.

| Contract                 | Description                                                                    |
| ------------------------ | ------------------------------------------------------------------------------ |
| `nebula-airdrop`         | Logic for NEB airdrops to LUNA stakers                                         |
| `nebula-cluster`         | Logic for mechanisms of individual clusters                                    |
| `nebula-cluster-factory` | Defines procedure for creating new Clusters                                    |
| `nebula-collector`       | Collects protocol fees and distributes it as rewards to NEB governance stakers |
| `nebula-community`       | Controls the funds in the governance-controlled community pool                 |
| `nebula-gov`             | Manages the decentralized governance functions of Nebula protocol              |
| `nebula-lp-staking`      | Manages NEB rewards for Cluster Token liquidity providers (LP)                 |

### Auxiliary

Some parts of Nebula such as a Cluster's penalty function or NEB incentive campaigns are also implemented using contracts but are not considered part of the protocol.
Nebula ships with a couple default ones, and their code is here.

| Contract                    | Description                                                      |
| --------------------------- | ---------------------------------------------------------------- |
| `nebula-penalty`            | Implementation of a Cluster Penalty Function, used by default    |
| `nebula-incentives`         | Implementation of a NEB incentive scheme for Terraswap arbitrage |
| `nebula-incentives-custody` | Custody contract for NEB incentive scheme                        |

### Testing

These contracts are solely used for testing purposes.

| Contract              | Description                                                     |
| --------------------- | --------------------------------------------------------------- |
| `nebula-dummy-oracle` | Cluster Oracle feeder that can be controlled during testing     |
| `terraswap-oracle`    | Oracle feeder that uses liquidity from Terraswap to feed prices |
