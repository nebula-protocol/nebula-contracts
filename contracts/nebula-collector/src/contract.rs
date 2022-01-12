#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, WasmMsg,
};

use crate::state::{read_config, store_config, Config};
use nebula_protocol::collector::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use nebula_protocol::gov::Cw20HookMsg as GovCw20HookMsg;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::{Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg};
use astroport::querier::{query_balance, query_pair_info, query_token_balance};
use cw20::Cw20ExecuteMsg;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            distribution_contract: deps.api.addr_validate(msg.distribution_contract.as_str())?,
            astroport_factory: deps.api.addr_validate(msg.astroport_factory.as_str())?,
            nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
            base_denom: msg.base_denom,
            owner: deps.api.addr_validate(msg.owner.as_str())?,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Convert { asset_token } => convert(deps, env, asset_token),
        ExecuteMsg::Distribute {} => distribute(deps, env),
        ExecuteMsg::UpdateConfig {
            distribution_contract,
            astroport_factory,
            nebula_token,
            base_denom,
            owner,
        } => update_config(
            deps,
            info,
            distribution_contract,
            astroport_factory,
            nebula_token,
            base_denom,
            owner,
        ),
    }
}

/// Convert
/// Anyone can execute convert function to swap
/// asset token => collateral token
/// collateral token => NEB token
pub fn convert(deps: DepsMut, env: Env, asset_token: String) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let astroport_factory_raw = config.astroport_factory;

    let pair_info: PairInfo = query_pair_info(
        &deps.querier,
        Addr::unchecked(astroport_factory_raw.to_string()),
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom.to_string(),
            },
            AssetInfo::Token {
                contract_addr: deps.api.addr_validate(asset_token.as_str())?,
            },
        ],
    )?;

    let messages: Vec<CosmosMsg>;
    if config.nebula_token == asset_token {
        // collateral token => nebula token
        let amount = query_balance(
            &deps.querier,
            env.contract.address,
            config.base_denom.to_string(),
        )?;
        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // deduct tax first
        let amount = (swap_asset.deduct_tax(&deps.querier)?).amount;
        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_info.contract_addr.to_string(),
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    amount,
                    ..swap_asset
                },
                max_spread: None,
                belief_price: None,
                to: None,
            })?,
            funds: vec![Coin {
                denom: config.base_denom,
                amount,
            }],
        })];
    } else {
        // asset token => collateral token
        let amount = query_token_balance(
            &deps.querier,
            Addr::unchecked(asset_token.to_string()),
            env.contract.address,
        )?;

        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: asset_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: pair_info.contract_addr.to_string(),
                amount,
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: None,
                    belief_price: None,
                    to: None,
                })?,
            })?,
            funds: vec![],
        })];
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        attr("action", "convert"),
        attr("asset_token", asset_token.as_str()),
    ]))
}

// Anyone can execute send function to receive staking token rewards
pub fn distribute(deps: DepsMut, env: Env) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    let amount = query_token_balance(
        &deps.querier,
        Addr::unchecked(config.nebula_token.to_string()),
        env.contract.address,
    )?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: config.distribution_contract.to_string(),
                amount,
                msg: to_binary(&GovCw20HookMsg::DepositReward {})?,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "distribute"),
            attr("amount", amount.to_string()),
        ]))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    distribution_contract: Option<String>,
    astroport_factory: Option<String>,
    nebula_token: Option<String>,
    base_denom: Option<String>,
    owner: Option<String>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    if let Some(distribution_contract) = distribution_contract {
        config.distribution_contract = deps.api.addr_validate(distribution_contract.as_str())?;
    }

    if let Some(astroport_factory) = astroport_factory {
        config.astroport_factory = deps.api.addr_validate(astroport_factory.as_str())?;
    }

    if let Some(nebula_token) = nebula_token {
        config.nebula_token = deps.api.addr_validate(nebula_token.as_str())?;
    }

    if let Some(base_denom) = base_denom {
        config.base_denom = base_denom;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        distribution_contract: state.distribution_contract.to_string(),
        astroport_factory: state.astroport_factory.to_string(),
        nebula_token: state.nebula_token.to_string(),
        base_denom: state.base_denom,
        owner: state.owner.to_string(),
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
