# Nebula LP Staking

The Staking Contract contains the logic for LP Token staking and reward distribution. Staking rewards for LP stakers come from the new NEB tokens generated at each block by the Factory Contract and are split between all combined staking pools. The new NEB tokens are distributed in proportion to size of staked LP tokens multiplied by the weight of that asset's staking pool.

- [Nebula LP Staking](#nebula-lp-staking)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [Receive](#receive)
    - [UpdateConfig](#updateconfig)
    - [RegisterAsset](#registerasset)
    - [Unbond](#unbond)
    - [Withdraw](#withdraw)
    - [AutoStake](#autostake)
  - [Receive Hook (CW20ReceiveMsg)](#receive-hook-cw20receivemsg)
    - [Bond](#bond)
    - [DepositReward](#depositreward)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [PoolInfo](#poolinfo)
    - [RewardInfo](#rewardinfo)

## InstantiateMsg

```json
{
    "owner": String,
    "nebula_token": String,
    "astroport_factory": String
}
```

- address of the owner of the `staking` contract
- `nebula_token`: contract address of Nebula Token (NEB)
- `astroport_factory`: address of the [Astroport](https://astroport.fi) [factory](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract

## ExecuteMsg

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
- `sender`: sender of the token transfer
- `msg`: Base64-encoded JSON of the Receive Hook

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": Option<String>
    }
}
```

- `owner`: address of the owner of the `staking` contract

### RegisterAsset

Registers a new staking pool for an asset token and associates the LP token with the staking pool.

```json
{
    "register_asset": {
        "asset_token": String,
        "staking_token": String
    }
}
```

- `asset_token`: contract address of cluster/NEB token
- `staking_token`: contract address of asset's corresponding LP token

### Unbond

Users can issue the unbond message at any time to remove their staked LP tokens from a staking position

```json
{
    "unbond": {
        "asset_token": String,
        "amount": Uint128
    }
}
```

- `asset_token`: contract address of cluster/NEB token
- `amount`: amount of LP tokens to unbond

### Withdraw

Withdraws a user's rewards for a specific staking position

```json
{
    "withdraw": {
        "asset_token": Option<String>
    }
}
```

- `asset_token`: contract address of asset token (staking pool identifier). If empty, withdraws all rewards from all staking pools involved.

### AutoStake

When providing liquidity in Nebula Protocol, asset pair is first sent to Staking contract, and it acts as a relay. When defined assets are sent to the contract, the contract provides liquidity to receive LP tokens and starts AutoStakeHook.

**Note: Executor of the transaction should first increase allowance to spend CW20 tokens**

```json
{
    "auto_stake": {
        "assets": [Asset; 2],
        "slippage_tolerance": Option<Decimal>
    }
}
```

- `assets`: information of assets that are provided into liquidity pool
- `slippage_tolerance`: maximum price slippage allowed to execute this transaction

## Receive Hook (CW20ReceiveMsg)

### Bond

Can be issued when the user sends LP Tokens to the Staking contract. The LP token must be recognized by the staking pool of the specified asset token.

**If you send LP Tokens to the Staking contract without issuing this hook, they will not be staked and will BE LOST FOREVER.**

```json
{
    "bond": {
        "asset_token": String
    }
}
```

- `asset_token`: contract address of asset token

### DepositReward

Can be issued when the user sends NEB tokens to the Staking contract, which will be used as rewards for the specified asset's staking pool. Used by Factory Contract to deposit newly minted NEB tokens.

```json
{
    "deposit_reward": {
        "rewards": Vec<(String, Uint128)>
    }
}
```

- `rewards`: list of reward info. vector of (asset_token, reward_amount)

## QueryMsg

### Config

```json
{
    "config": {}
}
```

### PoolInfo

Returns information related to a staking pool

```json
{
    "pool_info": {
        "asset_token": String
    }
}
```

- `asset_token`: contract address of asset token to query pool info for

### RewardInfo

Returns information related to a staker's reward info

```json
{
    "reward_info": {
        "staker_addr": String,
        "asset_token": Option<String>
    }
}
```

- `staker_addr`: address of the staker to query the reward info for
- `asset_token`: contract address of asset token to query pool info for. If empty, will return all revelant rewards for `staker_addr`