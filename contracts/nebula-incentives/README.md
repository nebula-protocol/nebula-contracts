# Nebula Incentives

- [Nebula Incentives](#nebula-incentives)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [UpdateConfig](#updateconfig)
    - [Receive](#receive)
    - [Withdraw](#withdraw)
    - [New Penalty Period](#new-penalty-period)
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

  "owner": String,
  "proxy": String,
  "custody": String,
  "nebula_token": String
}
```

- `owner`: address of the owner of the `incentives` contract
- `proxy`: address of the [`nebula-proxy`](../nebula-proxy/) contract
- `custody`: address of the [`nebula-incentives-custody`](../nebula-incentives-custody/) contract
- `nebula_token`: contract address of Nebula Token (NEB)

## ExecuteMsg

### UpdateConfig

Updates general contract parameters.

```json
{
  "update_config": {
    "owner": String
  }
}
```

- `owner`: address of the new owner of the `incentives` contract

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

Withdraws all incentives rewards

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

## Receive Hook (CW20ReceiveMsg)

### DepositRewards

Deposits incentives rewards. Callable by anyone.

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
