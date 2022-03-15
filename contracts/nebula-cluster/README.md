# Nebula Cluster

This contract stores information and functionalities related to the individual Nebula cluster. These includes the methods for rebalancing the cluster inventory, updating the cluster's target weights, and decommissioning the cluster. It also facilitates querying information related to the cluster, including the cluster's name/description/inventory composition, penalty and target oracle address, the underlying asset prices, and its outstanding balance.

- [Nebula Cluster](#nebula-cluster)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [UpdateConfig](#updateconfig)
    - [RebalanceCreate](#rebalancecreate)
    - [RebalanceRedeem](#rebalanceredeem)
    - [UpdateTarget](#updatetarget)
    - [Decommission](#decommission)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [Target](#target)
    - [ClusterState](#clusterstate)
    - [ClusterInfo](#clusterinfo)

## InstantiateMsg

```json
{
    "owner": String,
    "factory": String,
    "name": String,
    "description": String,
    "cluster_token": Option<String>,
    "pricing_oracle": String,
    "target_oracle": String,
    "target": Vec<Asset>,
    "penalty": String
}
```

- `owner`: address of the owner of the `cluster` contract
- `factory`: cluster factory contract address
- `name`: cluster name
- `description`: cluster description
- `cluster_token`: cluster token contract address
- `pricing_oracle`: address of price oracle use to calculate the prices of the cluster's inventory
- `target_oracle`: address of target oracle allowed to update the cluster's target weights
- `target`: cluster's target inventory asset weights
- `penalty`: penalty function contract address used by the cluster

## ExecuteMsg

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": Option<String>,
        "name": Option<String>,
        "description": Option<String>,
        "cluster_token": Option<String>,
        "pricing_oracle": Option<String>,
        "target_oracle:" Option<String>,
        "penalty": Option<String>,
        "target": Option<Vec<Asset>>
    }
}
```

- `owner`: address of the new owner of the `cluster` contract
- `name`: cluster name
- `description`: cluster description
- `cluster_token`: cluster token contract address
- `pricing_oracle`: address of price oracle use to calculate the prices of the cluster's inventory
- `target_oracle`: address of target oracle allowed to update the cluster's target weights
- `target`: cluster's target inventory asset weights
- `penalty`: penalty function contract address used by the cluster

### RebalanceCreate

Perform a [CREATE/mint](https://docs.neb.money/protocol/clusters.html#create-mint) operation on cluster, depositing the cluster's inventory assets and minting new cluster tokens

```json
{
    "rebalance_create": {
        "asset_amounts": Vec<Asset>,
        "min_tokens": Option<Uint128>
    }
}
```

- `asset_amounts`: asset amounts deposited for minting/rebalancing
- `min_tokens`: minimum cluster tokens to receive

### RebalanceRedeem

Perform a [`REDEEM/burn`](https://docs.neb.money/protocol/clusters.html#redeem-burn) operation on the cluster, burning cluster tokens in exchange for the cluster's inventory assets

```json
{
    "rebalance_redeem": {
        "asset_amounts": Option<Vec<Asset>>,
        "max_tokens": Uint128
    }
}
```

- `asset_amounts`: list of assets and asset weights to receive from burning the cluster tokens (putting this as `None` will do a pro-rata redeem based on the cluster's current inventory asset and weights)
- `min_tokens`: maximum amount of cluster tokens to spend to receive the inventory assets

### UpdateTarget

Update the target inventory asset weights (only callable by the cluster's owner or target oracle)

```json
{
    "update_target": {
        "target": Vec<Asset>
    }
}
```

- `target`: list of new target inventory asset and weights (`Asset` is an Astroport type defined [here](https://github.com/astroport-fi/astroport-core/blob/main/packages/astroport/src/asset.rs#L23))

### Decommission

Decommission the cluster, marking it as inactive and disabling all functionality excluding pro-rata redeeming of the cluster's inventory assets.

```json
{
    "decommission": {}
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

### Target

Returns the current target asset and weights saved in the contract.

```json
{
    "target": {}
}
```

### ClusterState

Returns the current cluster state, including

- the `outstanding_balance_tokens` (cluster token total supply),
- the latest `prices` of the inventory assets
- the amounts of each assets currently in the cluster's inventory
- the cluster's penalty contract address
- the cluster's token contract address
- the cluster's `target`
- the cluster's status

```json
{
    "cluster_state": {}
}
```

### ClusterInfo

Returns the current cluster state, including its name and description

```json
{
    "cluster_info": {}
}
```