use cosmwasm_std::{log, to_binary, Api, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, InitResponse, MigrateResponse, MigrateResult, Querier, StdResult, Storage, WasmMsg, from_binary, StdError, Uint128};

use crate::state::{read_config, store_config, Config, store_current_n, store_pool_info, PoolInfo};
use crate::rewards::{deposit_reward, record_penalty, withdraw_reward, increment_n};
use nebula_protocol::collector::{ConfigResponse, HandleMsg, InitMsg, MigrateMsg, QueryMsg, Cw20HookMsg};

use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, HandleMsg as TerraswapHandleMsg};
use terraswap::querier::{query_balance, query_pair_info, query_token_balance};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            distribution_contract: deps.api.canonical_address(&msg.distribution_contract)?,
            terraswap_factory: deps.api.canonical_address(&msg.terraswap_factory)?,
            nebula_token: deps.api.canonical_address(&msg.nebula_token)?,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    store_current_n(&mut deps.storage, 0)?;
    store_pool_info(&mut deps.storage, 0, &PoolInfo {
        n: 0,
        penalty_sum: Uint128::zero(),
        reward_sum: Uint128::zero(),
    })?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::_ResetOwner { owner } => try_reset_owner(deps, env, &owner),
        HandleMsg::Receive(msg) => receive_cw20(deps, env, msg),
        HandleMsg::Convert { asset_token } => convert(deps, env, asset_token),
        HandleMsg::Distribute {} => distribute(deps, env),
        HandleMsg::RecordPenalty {reward_owner, penalty_amount} => record_penalty(deps, env, reward_owner, penalty_amount),
        HandleMsg::Withdraw {} => withdraw_reward(deps, env),
        HandleMsg::NewPenaltyPeriod {} => new_penalty_period(deps, env),
    }
}


pub fn try_reset_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
) -> StdResult<HandleResponse> {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    let mut new_cfg = cfg.clone();
    new_cfg.owner = owner.clone();
    store_config(&mut deps.storage, &new_cfg)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "_try_reset_owner"),
        ],
        data: None,
    })
}

pub fn receive_cw20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cw20_msg: Cw20ReceiveMsg,
) -> HandleResult {
    if let Some(msg) = cw20_msg.msg {
        let config: Config = read_config(&deps.storage)?;

        match from_binary(&msg)? {
            Cw20HookMsg::DepositReward {} => {
                // only reward token contract can execute this message
                if config.nebula_token != deps.api.canonical_address(&env.message.sender)? {
                    return Err(StdError::unauthorized());
                }
                deposit_reward(deps, cw20_msg.amount)
            }
        }
    } else {
        Err(StdError::generic_err("data should be given"))
    }
}

/// Convert
/// Anyone can execute convert function to swap
/// asset token => collateral token
/// collateral token => NEB token
pub fn convert<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_token: HumanAddr,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let asset_token_raw = deps.api.canonical_address(&asset_token)?;
    let terraswap_factory_raw = deps.api.human_address(&config.terraswap_factory)?;

    let pair_info: PairInfo = query_pair_info(
        &deps,
        &terraswap_factory_raw,
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom.to_string(),
            },
            AssetInfo::Token {
                contract_addr: asset_token.clone(),
            },
        ],
    )?;

    let messages: Vec<CosmosMsg>;
    if config.nebula_token == asset_token_raw {
        // collateral token => nebula token
        let amount = query_balance(&deps, &env.contract.address, config.base_denom.to_string())?;
        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // deduct tax first
        let amount = (swap_asset.deduct_tax(&deps)?).amount;
        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.contract_addr,
            msg: to_binary(&TerraswapHandleMsg::Swap {
                offer_asset: Asset {
                    amount,
                    ..swap_asset
                },
                max_spread: None,
                belief_price: None,
                to: None,
            })?,
            send: vec![Coin {
                denom: config.base_denom,
                amount,
            }],
        })];
    } else {
        // asset token => collateral token
        let amount = query_token_balance(&deps, &asset_token, &env.contract.address)?;

        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset_token.clone(),
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: pair_info.contract_addr,
                amount,
                msg: Some(to_binary(&TerraswapCw20HookMsg::Swap {
                    max_spread: None,
                    belief_price: None,
                    to: None,
                })?),
            })?,
            send: vec![],
        })];
    }

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "convert"),
            log("asset_token", asset_token.as_str()),
        ],
        data: None,
    })
}

// Anyone can execute send function to receive staking token rewards
pub fn distribute<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let amount = query_token_balance(
        &deps,
        &deps.api.human_address(&config.nebula_token)?,
        &env.contract.address,
    )?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.nebula_token)?,
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: deps.api.human_address(&config.distribution_contract)?,
                amount,
                msg: Some(to_binary(&Cw20HookMsg::DepositReward {})?),
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "distribute"),
            log("amount", amount.to_string()),
        ],
        data: None,
    })
}

pub fn new_penalty_period<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let cfg = read_config(&deps.storage)?;

    if env.message.sender != cfg.owner {
        return Err(StdError::unauthorized());
    }

    increment_n(&mut deps.storage)?;
    Ok(
        HandleResponse {
            messages: vec![],
            log: vec![log("action", "new_penalty_period")],
            data: None,
        }
    )
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        distribution_contract: deps.api.human_address(&state.distribution_contract)?,
        terraswap_factory: deps.api.human_address(&state.terraswap_factory)?,
        nebula_token: deps.api.human_address(&state.nebula_token)?,
        base_denom: state.base_denom,
        owner: state.owner,
    };

    Ok(resp)
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> MigrateResult {
    Ok(MigrateResponse::default())
}
