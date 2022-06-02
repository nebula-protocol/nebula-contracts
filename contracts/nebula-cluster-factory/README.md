# Nebula Cluster Factory

The factory contract contains functionalities for creating and decommissioning Nebula clusters, as well as distributing LP staking rewards

- [Nebula Cluster Factory](#nebula-cluster-factory)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [PostInitialize](#postinitialize)
    - [UpdateConfig](#updateconfig)
    - [UpdateWeight](#updateweight)
    - [CreateCluster](#createcluster)
    - [PassCommand](#passcommand)
    - [DecommissionCluster](#decommissioncluster)
    - [Distribute](#distribute)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [ClusterExists](#clusterexists)
    - [ClusterList](#clusterlist)
    - [DistributionInfo](#distributioninfo)

## InstantiateMsg

```json
{
    "token_code_id": u64,
    "cluster_code_id": u64,
    "base_denom": String,
    "protocol_fee_rate": String,
    "dstribution_schedule": Vec<(u64, u64, Uint128)>
}
```

## ExecuteMsg

### PostInitialize

Adds necessary factory contract settings after the initialization

```json
{
    "post_initialize": {
        "owner": String,
        "astroport_factory": String,
        "nebula_token": String,
        "staking_contract": String,
        "commission_collector": String
    }
}
```

- `owner`: address of the owner of the `cluster-factory` contract
- `astroport_factory`: address of the [Astroport](https://astroport.fi) [`factory`](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract
- `nebula_token`: contract address of Nebula Token (NEB)
- `staking_contract`: address of the [`lp-staking`](../nebula-lp-staking/) contract
- `commission_collector`: address of the [`collector`](../nebula-collector/) contract used to send protocol fees to 

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": Option<String>,
        "token_code_id": Option<u64>,
        "cluster_code_id": Option<u64>,
        "distribution_schedule": Option<Vec<(u64, u64, Uint128)>>
    }
}
```

- `owner`: address of the new owner of the `cluster-factory` contract
- `token_code_id`: code ID of the CW20 token implementation to use for cluster tokens
- `cluster_code_id`: code ID of the cluster contract implementation to use
- `distribution_schedule`: distribution schedule for the LP staking NEB token rewards/incentives (list of `(start_time, end_time, distribution_amount)`)

### UpdateWeight

Updates the LP rewards distribution weight for a cluster token

```json
{
    "update_weight": {
        "asset_token": String,
        "weight": u32
    }
}
```

- `asset_token`: address of the Nebula token or cluster token
- `weight`: new reward distribution weight

### CreateCluster

Creates a new cluster. Only callable by the cluster factory's owner.

```json
{
    "create_cluster": {
        params: Params
    }
}
```

- `params`: params for the cluster being created

The `params` type has the following structure:

```json
{
    "name": String,
    "symbol": String,
    "description": String,
    "weight": Option<u32>,
    "penalty": Addr,
    "pricing_oracle": Addr,
    "target_oracle": Addr,
    "target": Vec<Asset>
}
```

- `name`: cluster name
- `description`: cluster description
- `weight`: weight for distributing LP rewards
- `penalty`: address of penalty functio contract to use with this cluster
- `pricing_oracle`: address of price oracle contract to use with this cluster
- `target_oracle`: address of the target oracle for this cluster
- `target`: initial cluster inventory target weights to use when first creating the cluster

### PassCommand

Allows the cluster factory to pass `ExecuteMsg` to other contracts. Used for contracts in which the cluster factory contract is the owner.

```json
{
    "pass_command": {
        "contract_addr": String,
        "msg": Binary
    }
}
```

- `contract_addr`: address of the contract to execute the `msg` on
- `msg`: binary-encoded contract `ExecuteMsg`

### DecommissionCluster

Decommissions a specific cluster, disabling all of its functionality execpet for pro-rata redeem (to allow users to withdraw assets from the cluster's inventory)

```json
{
    "decomission_cluster": {
        "cluster_contract": String,
        "cluster_token": String
    }
}
```

- `cluster_contract`: cluster contract address to decomission
- `cluster_token`: cluster token address to decomission

### Distribute

Distributes rewards to current Cluster-BASE_DENOM and NEB-BASE_DENOM LP stakers. This is done by calculating the amount of NEB tokens to distribute to each pool's stakers using:

- each pool's `weight`:
- the `distribution_schedule`
- time since the last reward distribution

```json
{
    "distribute": {}
}
```

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
    "config": {}
}
```

### ClusterExists

Returns whether a given cluster exists

```json
{
    "cluster_exists": {
        "cluster_address": String
    }
}
```

- `cluster_address`: address of the cluster contract to query

### ClusterList

Returns a list of all of the Nebula clusters (that are created through this factory contract) and its status (e.g. `active` or `decommissioned`).

```json
{
    "cluster_list": {}
}
```

### DistributionInfo

Returns the current LP staking reward distribution info

```json
{
    "distribution_info": {}
}
```