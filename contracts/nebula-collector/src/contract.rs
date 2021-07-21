use cosmwasm_std::{
    log, to_binary, Api, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, InitResponse, MigrateResponse, MigrateResult, Querier, StdError, StdResult, Storage,
    WasmMsg,
};

use crate::state::{read_config, store_config, Config};
use nebula_protocol::collector::{ConfigResponse, HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use nebula_protocol::gov::Cw20HookMsg as GovCw20HookMsg;

use cw20::Cw20HandleMsg;
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
            distribution_contract: msg.distribution_contract,
            terraswap_factory: msg.terraswap_factory,
            nebula_token: msg.nebula_token,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Convert { asset_token } => convert(deps, env, asset_token),
        HandleMsg::Distribute {} => distribute(deps, env),
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
    let terraswap_factory_raw = config.terraswap_factory;

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
    if config.nebula_token == asset_token {
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
    let amount = query_token_balance(&deps, &config.nebula_token, &env.contract.address)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token,
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: config.distribution_contract,
                amount,
                msg: Some(to_binary(&GovCw20HookMsg::DepositReward {})?),
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
        distribution_contract: state.distribution_contract,
        terraswap_factory: state.terraswap_factory,
        nebula_token: state.nebula_token,
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
