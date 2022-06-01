use cosmwasm_std::{
    attr, to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, Storage,
    Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::rewards::before_share_change;
use crate::state::{
    read_config, read_pool_info, rewards_read, rewards_store, store_pool_info, Config, PoolInfo,
    RewardInfo,
};
use nebula_protocol::staking::ExecuteMsg;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::ExecuteMsg as PairExecuteMsg;
use astroport::querier::{query_pair_info, query_token_balance};

use cw20::Cw20ExecuteMsg;

/// ## Description
/// Bonds the transferred LP token to the specified LP token pool.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **staker_addr** is an object of type [`Addr`] which the staker address.
///
/// - **asset_token** is an object of type [`Addr`] which is an address of
///     a cluster token contract.
///
/// - **amount** is an object of type [`Uint128`] which is the amount to bond.
pub fn bond(
    deps: DepsMut,
    _info: MessageInfo,
    staker_addr: Addr,
    asset_token: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Increase the staker's bond by the given amount
    _increase_bond_amount(deps.storage, &staker_addr, &asset_token, amount)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "bond"),
        attr("staker_addr", staker_addr.as_str()),
        attr("asset_token", asset_token.as_str()),
        attr("amount", amount.to_string()),
    ]))
}

/// ## Description
/// Unbonds the staked LP tokens of the staker for the given amount.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **staker_addr** is an object of type [`String`] is the staker address.
///
/// - **asset_token** is an object of type [`Addr`] which is an address of
///     a cluster token contract.
///
/// - **amount** is an object of type [`Uint128`] which is the amount to unbond.
pub fn unbond(
    deps: DepsMut,
    staker_addr: String,
    asset_token: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_staker_addr = deps.api.addr_validate(staker_addr.as_str())?;
    let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;

    // Decrease the staker's bond by the given amount
    let staking_token: Addr = _decrease_bond_amount(
        deps.storage,
        &validated_staker_addr,
        &validated_asset_token,
        amount,
    )?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: staking_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: validated_staker_addr.to_string(),
                amount,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "unbond"),
            attr("staker_addr", validated_staker_addr.as_str()),
            attr("asset_token", validated_asset_token.as_str()),
            attr("amount", amount.to_string()),
        ]))
}

/// ## Description
/// Provides liquidity and automatically stakes the LP tokens.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **assets** is an object of type [`Uint128`] which are assets for providing pool liquidity.
///
/// - **slippage_tolerance** is an object of type [`Option<Decimal>`] which is
///     the maximum percent of price movement when providing liquidity.
pub fn auto_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    assets: [Asset; 2],
    slippage_tolerance: Option<Decimal>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    let astroport_factory: Addr = config.astroport_factory;

    let mut native_asset_op: Option<Asset> = None;
    let mut token_info_op: Option<(Addr, Uint128)> = None;

    // Extract BASE_DENOM and CT from the given list `assets`
    for asset in assets.iter() {
        match asset.info.clone() {
            AssetInfo::NativeToken { .. } => {
                asset.assert_sent_native_token_balance(&info)?;
                native_asset_op = Some(asset.clone())
            }
            AssetInfo::Token { contract_addr } => {
                token_info_op = Some((contract_addr, asset.amount))
            }
        }
    }

    // Fail if one of them is missing
    let native_asset: Asset = match native_asset_op {
        Some(v) => v,
        None => return Err(ContractError::Missing("native asset".to_string())),
    };
    let (token_addr, token_amount) = match token_info_op {
        Some(v) => v,
        None => return Err(ContractError::Missing("token asset".to_string())),
    };

    // Query pair info to obtain Astroport pair contract address
    let asset_infos: [AssetInfo; 2] = [assets[0].info.clone(), assets[1].info.clone()];
    let astroport_pair: PairInfo = query_pair_info(&deps.querier, astroport_factory, &asset_infos)?;

    // Assert the token and LP token match with pool info
    let pool_info: PoolInfo = read_pool_info(deps.storage, &token_addr)?;

    if pool_info.staking_token != astroport_pair.liquidity_token {
        return Err(ContractError::Invalid("staking token".to_string()));
    }

    // Get current LP token amount staked in this LP staking contract
    // to later compute the received LP token amount
    let prev_staking_token_amount = query_token_balance(
        &deps.querier,
        astroport_pair.liquidity_token.clone(),
        env.contract.address.clone(),
    )?;

    // 1. Transfer token asset to LP staking contract
    // 2. Increase allowance of token for the Astroport pair contract
    // 3. Provide liquidity and get LP tokens
    // 4. Execute staking hook which stakes LP tokens in the name of the sender
    Ok(Response::new()
        .add_messages(vec![
            // Transfer cluster tokens from the message sender to this contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: token_amount,
                })?,
                funds: vec![],
            }),
            // Increase allowance for the Astroport pair contract by the this contract
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: astroport_pair.contract_addr.to_string(),
                    amount: token_amount,
                    expires: None,
                })?,
                funds: vec![],
            }),
            // Provide liquidity which gets LP tokens in return
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: astroport_pair.contract_addr.to_string(),
                msg: to_binary(&PairExecuteMsg::ProvideLiquidity {
                    assets: [
                        Asset {
                            amount: native_asset.amount,
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
                    auto_stake: None,
                    receiver: None,
                })?,
                funds: vec![Coin {
                    denom: native_asset.info.to_string(),
                    amount: native_asset.amount,
                }],
            }),
            // Execute staking hook which stakes LP tokens in the name of the sender
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::AutoStakeHook {
                    asset_token: token_addr.clone(),
                    staking_token: astroport_pair.liquidity_token,
                    staker_addr: info.sender,
                    prev_staking_token_amount,
                })?,
                funds: vec![],
            }),
        ])
        .add_attributes(vec![
            attr("action", "auto_stake"),
            attr("asset_token", token_addr.to_string()),
        ]))
}

/// ## Description
/// Stakes newly minted LP tokens into the LP staking pool.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_token** is an object of type [`Addr`] which is the address of
///     a cluster contract.
///
/// - **staking_token** is an object of type [`Addr`] which is the address of
///     a LP token contract corresponding to the cluster contract.
///
/// - **staker_addr** is an object of type [`Addr`] which is the staker address.
///
/// - **prev_staking_token_amount** is an object of type [`Uint128`] which is the LP token balance
///     of this contract before providing pool liquidity.
///
/// ## Executor
/// Only this contract can execute this.
pub fn auto_stake_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_token: Addr,
    staking_token: Addr,
    staker_addr: Addr,
    prev_staking_token_amount: Uint128,
) -> Result<Response, ContractError> {
    // Only can be called by itself
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // Compute newly LP tokens received
    // -- Compare with staking token amount before liquidity provision was executed
    let current_staking_token_amount =
        query_token_balance(&deps.querier, staking_token, env.contract.address)?;
    let amount_to_stake = current_staking_token_amount.checked_sub(prev_staking_token_amount)?;

    // Stake all LP tokens received
    bond(deps, info, staker_addr, asset_token, amount_to_stake)
}

/// ## Description
/// Increases the bonding amount of a staker.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **staker_addr** is a reference to an object of type [`Addr`] which is the staker address.
///
/// - **asset_token** is a reference to an object of type [`Addr`] which is an address
///     of a cluster token contract.
///
/// - **amount** is an object of type [`Uint128`] which is the amount to bond.
fn _increase_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &Addr,
    asset_token: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    // Get pool information and the staker information for this pool
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo = rewards_read(storage, staker_addr)
        .load(asset_token.as_bytes())
        .unwrap_or_else(|_| RewardInfo {
            index: Decimal::zero(),
            bond_amount: Uint128::zero(),
            pending_reward: Uint128::zero(),
        });

    // Withdraw reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Increase total bond amount in the pool
    pool_info.total_bond_amount += amount;
    // Increase staker's bond amount
    reward_info.bond_amount += amount;

    // Update rewards info
    rewards_store(storage, staker_addr).save(asset_token.as_bytes(), &reward_info)?;
    // Update pool info
    store_pool_info(storage, asset_token, &pool_info)?;

    Ok(())
}

/// ## Description
/// Decreases the bonding amount of a staker.
///
/// ## Params
/// - **storage** is a mutable reference to an object implementing trait [`Storage`].
///
/// - **staker_addr** is a reference to an object of type [`Addr`] which is the staker address.
///
/// - **asset_token** is a reference to an object of type [`Addr`] which is an address
///     of a cluster token contract.
///
/// - **amount** is an object of type [`Uint128`] which is the amount to bond.
fn _decrease_bond_amount(
    storage: &mut dyn Storage,
    staker_addr: &Addr,
    asset_token: &Addr,
    amount: Uint128,
) -> Result<Addr, ContractError> {
    // Get pool information and the staker information for this pool
    let mut pool_info: PoolInfo = read_pool_info(storage, asset_token)?;
    let mut reward_info: RewardInfo =
        rewards_read(storage, staker_addr).load(asset_token.as_bytes())?;

    if reward_info.bond_amount < amount {
        return Err(ContractError::Generic(
            "Cannot unbond more than bond amount".to_string(),
        ));
    }

    // Distribute reward to pending reward; before changing share
    before_share_change(&pool_info, &mut reward_info)?;

    // Decrease total bond amount in the pool
    pool_info.total_bond_amount = pool_info.total_bond_amount.checked_sub(amount)?;
    // Decrease staker's bond amount
    reward_info.bond_amount = reward_info.bond_amount.checked_sub(amount)?;

    // Update rewards info
    if reward_info.pending_reward.is_zero() && reward_info.bond_amount.is_zero() {
        rewards_store(storage, staker_addr).remove(asset_token.as_bytes());
    } else {
        rewards_store(storage, staker_addr).save(asset_token.as_bytes(), &reward_info)?;
    }

    // Update pool info
    store_pool_info(storage, asset_token, &pool_info)?;

    Ok(pool_info.staking_token)
}
