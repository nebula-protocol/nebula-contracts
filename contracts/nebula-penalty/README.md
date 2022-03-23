# Nebula Penalty

## InstantiateMsg

```json
{
    "owner": String,
    "penalty_params": PenaltyParams
}
```

- `owner`: address of the owner of the `penalty` contract
- `penalty_params`: the parameters for the penalty contract

## ExecuteMsg

### UpdateConfig

Updates general penalty contract parameters.

```json
{
    "update_config": {
        "owner": Option<String>,
        "penalty_params": Option<PenaltyParams>
    }
}
```

- `owner`: address of the new owner of the `penalty` contract
- `penalty_params`: new parameters for the penalty contract.

### PenaltyCreate

Updates penalty contract states, EMA and last block, after a create operation.

```json
{
    "penalty_create": {
        "block_height": u64,
        "cluster_token_supply": Uint128,
        "inventory": Vec<Uint128>,
        "create_asset_amounts": Vec<Uint128>,
        "asset_prices": Vec<String>,
        "target_weights": Vec<Uint128>
    }
}
```

- `block_height`: block height to compute the mint penalty at
- `cluster_token_supply` total supply for the cluster token
- `inventory`: current inventory of inventory assets in a cluster
- `create_asset_amounts`: provided asset amounts for minting cluster tokens
- `asset_prices`: prices of the inventory assets in a cluster
- `target_weights`: the cluster's current inventory asset weights

### PenaltyRedeem

Updates penalty contract states, EMA and last block, after a redeem operation.

```json
{
    "penalty_redeem": {
        "block_height": u64,
        "cluster_token_supply": Uint128,
        "inventory": Vec<Uint128>,
        "max_tokens": Uint128,
        "redeem_asset_amounts": Vec<Uint128>,
        "asset_prices": Vec<String>,
        "target_weights": Vec<Uint128>
    }
}
```

- `block_height`: the block height to compute the redeem penalty at
- `cluster_token_supply`: total supply for the cluster token
- `inventory`: current inventory of inventory assets in a cluster
- `max_tokens`: maximum amount of cluster tokens allowed to burn for pro-rata redeem
- `redeem_asset_amounts`: amounts expected to receive from burning cluster tokens
- `asset_prices`: latest prices of the inventory assets in a cluster
- `target_weights`: the cluster's current inventory asset weights

## QueryMsg

### PenaltyQueryCreate

Calculates the actual create amount after taking penalty into consideration

```json
{
    "penalty_query_create": {
        "block_height": u64,
        "cluster_token_supply": Uint128,
        "inventory": Vec<Uint128>,
        "create_asset_amounts": Vec<Uint128>,
        "asset_prices": Vec<String>,
        "target_weights": Vec<Uint128>
    }
}
```

- `block_height`: the block height to compute the redeem mint at
- `cluster_token_supply`: total supply for the cluster token
- `inventory`: current inventory of inventory assets in a cluster
- `create_asset_amounts`: provided asset amounts for minting cluster tokens
- `asset_prices`: prices of the inventory assets in a cluster
- `target_weights`: the cluster's current inventory asset weights

### PenaltyQueryRedeem

Calculates the actual redeem amount after taking penalty into consideration

```json
{
    "penalty_query_redeem": {
        "block_height": u64,
        "cluster_token_supply": Uint128,
        "inventory": Vec<Uint128>,
        "max_tokens": Uint128,
        "redeem_asset_amounts": Vec<Uint128>,
        "asset_prices": Vec<String>,
        "target_weights": Vec<Uint128>
    }
}
```

- `block_height`: the block height to compute the redeem mint at
- `cluster_token_supply`: total supply for the cluster token
- `inventory`: current inventory of inventory assets in a cluster
- `max_tokens`: maximum amount of cluster tokens allowed to burn for pro-rata redeem
- `redeem_asset_amounts`: amounts expected to receive from burning cluster tokens
- `asset_prices`: prices of the inventory assets in a cluster
- `target_weights`: the cluster's current inventory asset weights

### Params

Returns current penalty parameters

```json
{
    "params": {}
}
```

### Config

Returns general contract parameters

```json
{
    "config": {}
}
```