# Nebula Proxy

- [Nebula Proxy](#nebula-proxy)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [ArbClusterCreate](#arbclustercreate)
    - [ArbClusterRedeem](#arbclusterredeem)
    - [IncentivesCreate](#incentivescreate)
    - [IncentivesRedeem](#incentivesredeem)
  - [QueryMsg](#querymsg)
    - [Config](#config)

## InstantiateMsg

```json
{
  "factory": String,
  "incentives": String,
  "astroport_factory": String,
  "nebula_token": String,
  "base_denom": String,
  "owner": String
}
```

- `factory`: address of the [`cluster-factory`](../nebula-cluster-factory/) contract
- `incentives`: address of the [`nebula-incentives`](../nebula-incentives/) contract
- `astroport_factory`: address of the [Astroport](https://astroport.fi) [factory](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract
- `nebula_token`: contract address of Nebula Token (NEB)
- `base_denom`: contract's base denomination (usually `uusd`)
- `owner`: address of the owner of the `incentives` contract

## ExecuteMsg

### UpdateConfig

Updates general contract parameters.

```json
{
  "update_config": {
    "owner": Option<String>,
    "incentives": Option<Option<String>>
  }
}
```

- `owner`: address of the new owner of the `proxy` contract
- `incentives`: address of the new [`nebula-incentives`](../nebula-incentives/) contract. Empty if want to remove the current incentives contract

### ArbClusterCreate

Initiates an arbitrage mint transaction. This comprises of:

- using `assets` to mint new `cluster_contract` cluster tokens
- selling the minted tokens on Astroport, controlling the slippage with the `min_ust` option

```json
{
  "arb_cluster_create": {
    "cluster_contract": String,
    "assets": Vec<Asset>,
    "min_ust": Option<Uint128>
  }
}
```

- `cluter_contract`: address of the clsuter contract to mint tokens from
- `assets`: list of assets and amounts to use to mint the cluster tokens
- `min_ust`: minimum amount of UST expected to receive back when selling the minted cluster tokens on Astroport (for slippage control)

### ArbClusterRedeem

Initiates an arbitrage burn transaction. This comprises of:

- Using UST to buy cluster tokens from the CT-UST Astroport pool, controlling the slippage with `min_cluster`
- Burning the swapped cluster tokens for the cluster's inventory asset tokens

```json
{
  "arb_cluster_redeem": {
    "cluster_contract": String,
    "asset": Asset,
    "min_cluster": Option<Uint128>
  }
}
```

- `cluster_contract`: address of the clsuter contract to buy from Astroport and burn tokens from
- `asset`: asset to use to buy cluster tokens from Astroport (always `uusd`)
- `min_cluster`: minimum amount of cluster tokens expected to receive back when buying from Astroport pool (for slippage control)

### IncentivesCreate

NEB-incentivized version of the [`cluster`](../nebula-cluster/)'s mint/CREATE

```json
{
  "incentives_create": {
    "cluster_contract": String,
    "asset_amounts": Vec<Asset>,
    "min_tokens": Option<Uint128>
  }
}
```

- `cluster_contract`: cluster contract to do the mint/create transaction on
- `asset_amount`: list of assets to use to mint the cluster tokens with
- `min_tokens` minimum expected cluster tokens received from the mint (transaction will fail is output is below)

### IncentivesRedeem

NEB-incentivized version of the [`cluster`](../nebula-cluster/)'s burn/REDEEM

```json
{
  "incentives_redeem": {
    "cluster_contract": String,
    "max_tokens": Uint128,
    "asset_amounts": Option<Vec<Asset>>
  }
}
```

- `cluster_contract`: cluster contract to do the burn/redeem transaction on
- `max_tokens`: maximum amount of cluster tokens expected to be burnede to receive the `asset_amounts` out (transaction will fail if more than `max_tokens` cluster tokens are required)
- `asset_amounts` assets amount to receive back from the burn/redeem

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
  "config": {}
}
```
