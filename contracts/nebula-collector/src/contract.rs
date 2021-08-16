use cosmwasm_std::{
    entry_point, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, HumanAddr, MessageInfo,
    Response, StdResult, WasmMsg,
};

use crate::state::{read_config, store_config, Config};
use nebula_protocol::collector::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use nebula_protocol::gov::Cw20HookMsg as GovCw20HookMsg;

use cw20::Cw20ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo, PairInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, ExecuteMsg as TerraswapExecuteMsg};
use terraswap::querier::{query_balance, query_pair_info, query_token_balance};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            distribution_contract: msg.distribution_contract,
            terraswap_factory: msg.terraswap_factory,
            nebula_token: msg.nebula_token,
            base_denom: msg.base_denom,
            owner: msg.owner,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Convert { asset_token } => convert(deps, env, asset_token),
        ExecuteMsg::Distribute {} => distribute(deps, env),
    }
}

/// Convert
/// Anyone can execute convert function to swap
/// asset token => collateral token
/// collateral token => NEB token
pub fn convert(deps: DepsMut, env: Env, asset_token: HumanAddr) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
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
            msg: to_binary(&TerraswapExecuteMsg::Swap {
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
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: pair_info.contract_addr,
                amount,
                msg: Some(to_binary(&TerraswapCw20HookMsg::Swap {
                    max_spread: None,
                    belief_price: None,
                    to: None,
                })?),
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
    let amount = query_token_balance(&deps, &config.nebula_token, &env.contract.address)?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: config.distribution_contract,
                amount,
                msg: Some(to_binary(&GovCw20HookMsg::DepositReward {})?),
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "distribute"),
            attr("amount", amount.to_string()),
        ]))
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
        distribution_contract: state.distribution_contract,
        terraswap_factory: state.terraswap_factory,
        nebula_token: state.nebula_token,
        base_denom: state.base_denom,
        owner: state.owner,
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
