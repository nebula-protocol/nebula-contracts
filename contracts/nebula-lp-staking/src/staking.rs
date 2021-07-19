use cosmwasm_std::{
    log, to_binary, Api, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse, HandleResult, 
    HumanAddr, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use terraswap::asset::{Asset, AssetInfo, PairInfo};
use nebula_protocol::staking::HandleMsg;
use crate::rewards::before_share_change;
use crate::state::{
    read_pool_info, rewards_read, rewards_store, store_pool_info, 
    read_config, Config, PoolInfo, RewardInfo,
};

use terraswap::pair::HandleMsg as PairHandleMsg;
use terraswap::querier::{query_pair_info, query_token_balance};


use cw20::Cw20HandleMsg;

pub fn bond<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    staker_addr: HumanAddr,
    asset_token: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    _increase_bond_amount(&mut deps.storage, &staker_addr, &asset_token, amount)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "bond"),
            log("staker_addr", staker_addr.as_str()),
            log("asset_token", asset_token.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn unbond<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    staker_addr: HumanAddr,
    asset_token: HumanAddr,
    amount: Uint128,
) -> HandleResult {
    let staking_token: HumanAddr =
        _decrease_bond_amount(&mut deps.storage, &staker_addr, &asset_token, amount)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_token,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: staker_addr.clone(),
                amount,
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "unbond"),
            log("staker_addr", staker_addr.as_str()),
            log("asset_token", asset_token.as_str()),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn auto_stake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let terraswap_factory: HumanAddr = config.terraswap_factory;

    let mut native_asset_op: Option<Asset> = None;
    let mut token_info_op: Option<(HumanAddr, Uint128)> = None;
    for asset in assets.iter() {
        match asset.info.clone() {
            AssetInfo::NativeToken { .. } => {
                asset.assert_sent_native_token_balance(&env)?;
                native_asset_op = Some(asset.clone())
            }
            AssetInfo::Token { contract_addr } => {
                token_info_op = Some((contract_addr, asset.amount))
            }
        }
    }

    // will fail if one of them is missing
    let native_asset: Asset = match native_asset_op {
        Some(v) => v,
        None => return Err(StdError::generic_err("Missing native asset")),
    };
    let (token_addr, token_amount) = match token_info_op {
        Some(v) => v,
        None => return Err(StdError::generic_err("Missing token asset")),
    };

    // query pair info to obtain pair contract address
    let asset_infos: [AssetInfo; 2] = [assets[0].info.clone(), assets[1].info.clone()];
    let terraswap_pair: PairInfo = query_pair_info(deps, &terraswap_factory, &asset_infos)?;

    // assert the token and lp token match with pool info
    let pool_info: PoolInfo =
        read_pool_info(&deps.storage, &token_addr)?;

    if pool_info.staking_token
        != terraswap_pair.liquidity_token
    {
        return Err(StdError::generic_err("Invalid staking token"));
    }

    // get current lp token amount to later compute the recived amount
    let prev_staking_token_amount = query_token_balance(
        &deps,
        &terraswap_pair.liquidity_token,
        &env.contract.address,
    )?;

    // compute tax
    let tax_amount: Uint128 = native_asset.compute_tax(deps)?;

    // 1. Transfer token asset to staking contract
    // 2. Increase allowance of token for pair contract
    // 3. Provide liquidity
    // 4. Execute staking hook, will stake in the name of the sender
    Ok(HandleResponse {
        messages: vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.clone(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: token_amount,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.clone(),
                msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                    spender: terraswap_pair.contract_addr.clone(),
                    amount: token_amount,
                    expires: None,
                })?,
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: terraswap_pair.contract_addr,
                msg: to_binary(&PairHandleMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            amount: (native_asset.amount.clone() - tax_amount)?,
                            info: native_asset.info.clone(),
                        },
                        Asset {
                            amount: token_amount,
                            info: AssetInfo::Token {
                                contract_addr: token_addr.clone(),
                            },
                        },
                    ],
                    slippage_tolerance,
                })?,
                send: vec![Coin {
                    denom: native_asset.info.to_string(),
                    amount: (native_asset.amount - tax_amount)?,
                }],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::AutoStakeHook {
                    asset_token: token_addr.clone(),
                    staking_token: terraswap_pair.liquidity_token,
                    staker_addr: env.message.sender,
                    prev_staking_token_amount,
                })?,
                send: vec![],
            }),
        ],
        log: vec![
            log("action", "auto_stake"),
            log("asset_token", token_addr.to_string()),
            log("tax_amount", tax_amount.to_string()),
        ],
        data: None,
    })
}

pub fn auto_stake_hook<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_token: HumanAddr,
    staking_token: HumanAddr,
    staker_addr: HumanAddr,
    prev_staking_token_amount: Uint128,
) -> HandleResult {
    // only can be called by itself
    if env.message.sender != env.contract.address {
        return Err(StdError::unauthorized());
    }

    // stake all lp tokens received, compare with staking token amount before liquidity provision was executed
    let current_staking_token_amount =
        query_token_balance(&deps, &staking_token, &env.contract.address)?;
    let amount_to_stake = (current_staking_token_amount - prev_staking_token_amount)?;

    bond(deps, env, staker_addr, asset_token, amount_to_stake)
}

fn _increase_bond_amount<S: Storage>(
    storage: &mut S,
    staker_addr: &HumanAddr,
    asset_token: &HumanAddr,
    amount: Uint128,
) -> StdResult<()> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr)
        .load(asset_token.as_str().as_bytes())
        .unwrap_or_else(|_| RewardInfo {
            index: Decimal::zero(),
            bond_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
        });

    // Withdraw reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Increase total short or bond amount
    pool_info.total_bond_amount += amount;

    reward_info.bond_amount += amount;
    rewards_store(storage, &staker_addr).save(asset_token.as_str().as_bytes(), &reward_info)?;
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(())
}

fn _decrease_bond_amount<S: Storage>(
    storage: &mut S,
    staker_addr: &HumanAddr,
    asset_token: &HumanAddr,
    amount: Uint128,
) -> StdResult<HumanAddr> {
    let mut pool_info: PoolInfo = read_pool_info(storage, &asset_token)?;
    let mut reward_info: RewardInfo =
        rewards_read(storage, &staker_addr).load(asset_token.as_str().as_bytes())?;

    if reward_info.bond_amount < amount {
        return Err(StdError::generic_err("Cannot unbond more than bond amount"));
    }

    // Distribute reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Decrease total short or bond amount
    pool_info.total_bond_amount = (pool_info.total_bond_amount - amount)?;

    reward_info.bond_amount = (reward_info.bond_amount - amount)?;

    // Update rewards info
    if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
        rewards_store(storage, &staker_addr).remove(asset_token.as_str().as_bytes());
    } else {
        rewards_store(storage, &staker_addr).save(asset_token.as_str().as_bytes(), &reward_info)?;
    }

    // Update pool info
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(pool_info.staking_token)
}
