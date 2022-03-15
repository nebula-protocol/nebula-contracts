# Nebula Incentives


- [Nebula Incentives](#nebula-incentives)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [UpdateOwner](#updateowner)
    - [Receive](#receive)
    - [Withdraw](#withdraw)
    - [New Penalty Period](#new-penalty-period)
    - [ArbClusterCreate](#arbclustercreate)
    - [ArbClusterRedeem](#arbclusterredeem)
    - [IncentivesCreate](#incentivescreate)
    - [IncentivesRedeem](#incentivesredeem)
  - [Receive Hook (CW20ReceiveMsg)](#receive-hook-cw20receivemsg)
    - [DepositRewards](#depositrewards)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [PenaltyPeriod](#penaltyperiod)
    - [PoolInfo](#poolinfo)
    - [CurrentContributorInfo](#currentcontributorinfo)
    - [ContributorPendingRewards](#contributorpendingrewards)

## InstantiateMsg

```json
{
  "factory": String,
  "custody": String,
  "astroport_factory": String,
  "nebula_token": String,
  "base_denom": String,
  "owner": String
}
```

- `factory`: address of the [`cluster-factory`](../nebula-cluster-factory/) contract
- `custody`: address of the [`nebula-incentives-custody`](../nebula-incentives-custody/) contract
- `astroport_factory`: address of the [Astroport](https://astroport.fi) [factory](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract
- `nebula_token`: contract address of Nebula Token (NEB)
- `base_denom`: contract's base denomination (usually `uusd`)
- `owner`: address of the owner of the `incentives` contract

## ExecuteMsg

### UpdateOwner

```json
{
  "update_owner": {
    "owner": String
  }
}
```

### Receive

Can be called during a CW20 token transfer when the Gov contract is the recipient. Allows the token transfer to execute a [Receive Hook](#receive-hook-cw20receivemsg) as a subsequent action within the same transaction.

```json
{
    "receive": {
        "amount": Uint128,
        "sender": String,
        "msg": Option<Binary>
    }
}
```

- `amount`: amount of tokens received
- `sender`: Sender of the token transfer
- `msg`: Base64-encoded JSON of the Receive Hook

### Withdraw

Withdraw all incentives rewards

```json
{
  "withdraw": {}
}
```

### New Penalty Period

Initiates a new incentives penalty period and make the reward from the previous period claimable

```json
{
  "new_penalty_period": {}
}
```

### ArbClusterCreate

Initiate an arbitrage mint transaction. This comprises of:

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

Initiate an arbitrage burn transaction. This comprises of:

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

## Receive Hook (CW20ReceiveMsg)

### DepositRewards

Deposit incentives rewards. Callable by anyone.

```json
{
  "deposit_rewards": {
    "rewards": Vec<(u16, String, Uint128)>
  }
}
```

- `rewards`: The rewards distribution

The vector struct is:

- `pool_type` (u16): either REBALANCE (0) or ARBITRAGE (1)
- `cluster_contract` (String): cluster contract address
- `amount`: Amount of NEB rewards to deposit for this `pool_type`/`cluster_contract` combination

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
  "config": {}
}
```

### PenaltyPeriod

Returns the current penalty period

```json
{
  "penalty_period": {}
}
```

### PoolInfo

Returns information related to a specific pool

```json
{
  "pool_info": {
    "pool_type": u16,
    "cluster_address": String,
    "n": Option<u64>
  }
}
```

- `pool_type`: either REBALANCE (0) or ARBITRAGE (1)
- `cluster_address`: address of the cluster to query the pool info for
- `n`: penalty period to query the pool info for

### CurrentContributorInfo

Returns the information related to an incentives contributor

```json
{
  "current_contributor_info": {
    "pool_type": u16,
    "contributor_address": String,
    "cluster_address": String
  }
}
```

- `pool_type`: either REBALANCE (0) or ARBITRAGE (1)
- `contributor_address`: address of the contributor address to query the info for
- `cluster_address`: address of the cluster to query the contribution info for

### ContributorPendingRewards

Returns the information on the pending rewards for a contributor

```json
{
  "contributor_pending_rewards": {
    "contributor_address": String
  }
}
```

- `contributor_address`: address of the contributor to query the pending rewards info for