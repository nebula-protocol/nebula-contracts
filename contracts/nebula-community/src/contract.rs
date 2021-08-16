use crate::state::{read_config, store_config, Config};

use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, HumanAddr, MessageInfo,
    MigrateResult, Response, StdError, StdResult, Uint128, WasmMsg,
};

use nebula_protocol::community::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use cw20::Cw20ExecuteMsg;

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
            owner: msg.owner,
            nebula_token: msg.nebula_token,
            spend_limit: msg.spend_limit,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { owner, spend_limit } => {
            update_config(deps, env, owner, spend_limit)
        }
        ExecuteMsg::Spend { recipient, amount } => spend(deps, env, recipient, amount),
    }
}

pub fn update_config(
    deps: DepsMut,
    env: Env,
    owner: Option<HumanAddr>,
    spend_limit: Option<Uint128>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    if let Some(spend_limit) = spend_limit {
        config.spend_limit = spend_limit;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// Spend
/// Owner can execute spend operation to send
/// `amount` of NEB token to `recipient` for community purpose
pub fn spend(
    deps: DepsMut,
    env: Env,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    if config.spend_limit < amount {
        return Err(StdError::generic_err("Cannot spend more than spend_limit"));
    }

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token,
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: recipient.clone(),
                amount,
            })?,
        })])
        .add_attributes(
            attributes:
                vec![
                    attr("action", "spend"),
                    attr("recipient", recipient),
                    attr("amount", amount),
                ],
        ))
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
        owner: state.owner,
        nebula_token: state.nebula_token,
        spend_limit: state.spend_limit,
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
