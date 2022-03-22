# Nebula Penalty

## InstantiateMsg

```json
{
    "owner": String,
    "penalty_params": PenaltyParams
}
```

## ExecuteMsg

### UpdateConfig

```json
{
    "update_config": {
        "owner": Option<String>,
        "penalty_params": Option<PenaltyParams>
    }
}
```

### PenaltyCreate

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

### PenaltyRedeem

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

## QueryMsg

### PenaltyQueryCreate

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

### PenaltyQueryRedeem

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

### Params


```json
{
    "params": {}
}
```