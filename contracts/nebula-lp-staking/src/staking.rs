use cosmwasm_std::{
    attr, to_binary, Addr, CanonicalAddr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::rewards::before_share_change;
use crate::state::{
    read_config, read_pool_info, rewards_read, rewards_store, store_pool_info, Config, PoolInfo,
    RewardInfo,
};
use nebula_protocol::staking::ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo, PairInfo};

use terraswap::pair::ExecuteMsg as PairExecuteMsg;
use terraswap::querier::{query_pair_info, query_token_balance};

use cw20::Cw20ExecuteMsg;

pub fn bond(
    deps: DepsMut,
    _info: MessageInfo,
    staker_addr: Addr,
    asset_token: Addr,
    amount: Uint128,
) -> StdResult<Response> {
    let staker_addr_raw: CanonicalAddr = deps.api.addr_canonicalize(staker_addr.as_str())?;
    let asset_token_raw: CanonicalAddr = deps.api.addr_canonicalize(asset_token.as_str())?;
    _increase_bond_amount(deps.storage, &staker_addr_raw, &asset_token_raw, amount)?;
    Ok(Response::new().add_attributes(vec![
        attr("action", "bond"),
        attr("staker_addr", staker_addr.as_str()),
        attr("asset_token", asset_token.as_str()),
        attr("amount", amount.to_string()),
    ]))
}

pub fn unbond(
    deps: DepsMut,
    staker_addr: String,
    asset_token: String,
    amount: Uint128,
) -> StdResult<Response> {
    let staker_addr_raw: CanonicalAddr = deps.api.addr_canonicalize(staker_addr.as_str())?;
    let asset_token_raw: CanonicalAddr = deps.api.addr_canonicalize(asset_token.as_str())?;
    let staking_token: CanonicalAddr =
        _decrease_bond_amount(deps.storage, &staker_addr_raw, &asset_token_raw, amount)?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.addr_humanize(&staking_token)?.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: staker_addr.to_string(),
                amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "unbond"),
            attr("staker_addr", staker_addr.as_str()),
            attr("asset_token", asset_token.as_str()),
            attr("amount", amount.to_string()),
        ]))
}

pub fn auto_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let terraswap_factory: Addr = deps.api.addr_humanize(&config.terraswap_factory)?;

    let mut native_asset_op: Option<Asset> = None;
    let mut token_info_op: Option<(Addr, Uint128)> = None;
    for asset in assets.iter() {
        match asset.info.clone() {
            AssetInfo::NativeToken { .. } => {
                asset.assert_sent_native_token_balance(&info)?;
                native_asset_op = Some(asset.clone())
            }
            AssetInfo::Token { contract_addr } => {
                token_info_op = Some((deps.api.addr_validate(&contract_addr)?, asset.amount))
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
    let terraswap_pair: PairInfo = query_pair_info(&deps.querier, terraswap_factory, &asset_infos)?;

    // assert the token and lp token match with pool info
    let pool_info: PoolInfo = read_pool_info(
        deps.storage,
        &deps.api.addr_canonicalize(token_addr.as_str())?,
    )?;

    if pool_info.staking_token
        != deps
            .api
            .addr_canonicalize(terraswap_pair.liquidity_token.as_str())?
    {
        return Err(StdError::generic_err("Invalid staking token"));
    }

    // get current lp token amount to later compute the recived amount
    let prev_staking_token_amount = query_token_balance(
        &deps.querier,
        deps.api.addr_validate(&terraswap_pair.liquidity_token)?,
        env.contract.address.clone(),
    )?;

    // compute tax
    let tax_amount: Uint128 = native_asset.compute_tax(&deps.querier)?;

    // 1. Transfer token asset to staking contract
    // 2. Increase allowance of token for pair contract
    // 3. Provide liquidity
    // 4. Execute staking hook, will stake in the name of the sender
    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: token_amount,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: terraswap_pair.contract_addr.to_string(),
                    amount: token_amount,
                    expires: None,
                })?,
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: terraswap_pair.contract_addr.to_string(),
                msg: to_binary(&PairExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            amount: native_asset.amount.checked_sub(tax_amount)?,
                            info: native_asset.info.clone(),
                        },
                        Asset {
                            amount: token_amount,
                            info: AssetInfo::Token {
                                contract_addr: token_addr.to_string(),
                            },
                        },
                    ],
                    slippage_tolerance,
                    receiver: None,
                })?,
                funds: vec![Coin {
                    denom: native_asset.info.to_string(),
                    amount: native_asset.amount.checked_sub(tax_amount)?,
                }],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::AutoStakeHook {
                    asset_token: token_addr.to_string(),
                    staking_token: terraswap_pair.liquidity_token.clone(),
                    staker_addr: info.sender.to_string(),
                    prev_staking_token_amount,
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "auto_stake"),
            attr("asset_token", token_addr.to_string()),
            attr("tax_amount", tax_amount.to_string()),
        ]))
}

pub fn auto_stake_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_token: Addr,
    staking_token: Addr,
    staker_addr: Addr,
    prev_staking_token_amount: Uint128,
) -> StdResult<Response> {
    // only can be called by itself
    if info.sender.to_string() != env.contract.address {
        return Err(StdError::generic_err("unauthorized"));
    }

    // stake all lp tokens received, compare with staking token amount before liquidity provision was executed
    let current_staking_token_amount = query_token_balance(
        &deps.querier,
        Addr::unchecked(staking_token.to_string()),
        env.contract.address,
    )?;
    let amount_to_stake = current_staking_token_amount.checked_sub(prev_staking_token_amount)?;

    bond(deps, info, staker_addr, asset_token, amount_to_stake)
}

fn _increase_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_token: &CanonicalAddr,
    amount: Uint128,
) -> StdResult<()> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr)
        .load(asset_token.as_slice())
        .unwrap_or_else(|_| RewardInfo {
            index: Decimal::zero(),
            bond_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
        });

    // Withdraw reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Increase bond amount
    pool_info.total_bond_amount += amount;

    reward_info.bond_amount += amount;
    rewards_store(storage, &staker_addr).save(asset_token.as_slice(), &reward_info)?;
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(())
}

fn _decrease_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &CanonicalAddr,
    asset_token: &CanonicalAddr,
    amount: Uint128,
) -> StdResult<CanonicalAddr> {
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo =
        rewards_read(storage, &staker_addr).load(asset_token.as_slice())?;

    if reward_info.bond_amount < amount {
        return Err(StdError::generic_err("Cannot unbond more than bond amount"));
    }

    // Distribute reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Decrease bond amount
    pool_info.total_bond_amount = pool_info.total_bond_amount.checked_sub(amount)?;

    reward_info.bond_amount = reward_info.bond_amount.checked_sub(amount)?;

    // Update rewards info
    if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
        rewards_store(storage, &staker_addr).remove(asset_token.as_slice());
    } else {
        rewards_store(storage, &staker_addr).save(asset_token.as_slice(), &reward_info)?;
    }

    // Update pool info
    store_pool_info(storage, &asset_token, &pool_info)?;

    Ok(pool_info.staking_token)
}
