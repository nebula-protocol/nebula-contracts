#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    WasmMsg,
};

use crate::error::ContractError;
use crate::state::{read_config, store_config, Config};
use nebula_protocol::collector::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use nebula_protocol::gov::Cw20HookMsg as GovCw20HookMsg;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::pair::{Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg};
use astroport::querier::{query_balance, query_pair_info, query_token_balance};
use cw20::Cw20ExecuteMsg;

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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

/// ## Description
/// Exposes all the execute functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             distribution_contract,
///             astroport_factory,
///             nebula_token,
///             base_denom,
///             owner,
///         }** Updates general collector contract parameters.
///
/// - **ExecuteMsg::Convert {
///             asset_token,
///         }** Swaps UST to NEB or any CW20 to UST.
///
/// - **ExecuteMsg::Distribute {}** sends all collected fee to the Governance contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
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
        ExecuteMsg::Convert { asset_token } => convert(deps, env, asset_token),
        ExecuteMsg::Distribute {} => distribute(deps, env),
    }
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **distribution_contract** is an object of type [`Option<String>`] which is an address
///     of the contract to distribute rewards, supposedly the Governance contract.
///
/// - **astroport_factory** is an object of type [`Option<String>`] which is an address
///     of Astroport factory contract.
///
/// - **nebula_token** is an object of type [`Option<String>`] which is an address of
///     Nebula token contract.
///
/// - **base_denom** is an object of type [`Option<String>`] which is the base denom
///     for all operations, supposedly UST.
///
/// - **owner** is an object of type [`Option<String>`] which is an owner address to update.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    distribution_contract: Option<String>,
    astroport_factory: Option<String>,
    nebula_token: Option<String>,
    base_denom: Option<String>,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // Validate address format
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }

    if let Some(distribution_contract) = distribution_contract {
        // Validate address format
        config.distribution_contract = deps.api.addr_validate(distribution_contract.as_str())?;
    }

    if let Some(astroport_factory) = astroport_factory {
        // Validate address format
        config.astroport_factory = deps.api.addr_validate(astroport_factory.as_str())?;
    }

    if let Some(nebula_token) = nebula_token {
        // Validate address format
        config.nebula_token = deps.api.addr_validate(nebula_token.as_str())?;
    }

    if let Some(base_denom) = base_denom {
        config.base_denom = base_denom;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Swaps the given asset to another. If `asset_token` is
/// - Nebula token contract, trade all UST in the contract to Nebula tokens.
/// - Other CW20 assets, trade all of those assets in the contract to UST.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **asset_token** is an object of type [`String`] which is an address of CW20 token contract.
pub fn convert(deps: DepsMut, env: Env, asset_token: String) -> Result<Response, ContractError> {
    let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
    let config: Config = read_config(deps.storage)?;
    let astroport_factory_raw = config.astroport_factory;

    // Get a pair info in Astroport between UST and the given CW20 token
    let pair_info: PairInfo = query_pair_info(
        &deps.querier,
        astroport_factory_raw,
        &[
            AssetInfo::NativeToken {
                denom: config.base_denom.to_string(),
            },
            AssetInfo::Token {
                contract_addr: validated_asset_token.clone(),
            },
        ],
    )?;

    let messages: Vec<CosmosMsg>;
    if config.nebula_token == validated_asset_token {
        // If the given asset if Nebula, trade UST => Nebula

        // Query the current UST balance of the collector contract
        let amount = query_balance(
            &deps.querier,
            env.contract.address,
            config.base_denom.to_string(),
        )?;
        // Determine UST to be the trade offer
        let swap_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: config.base_denom.clone(),
            },
            amount,
        };

        // Deduct tax first
        let amount = (swap_asset.deduct_tax(&deps.querier)?).amount;

        // Execute swap from UST to NEB on Astroport UST-NEB pair contract
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
        // If the given asset if other CW20, trade the given CW20 => UST

        // Query the given CW20 balance of the collector contract
        let amount = query_token_balance(
            &deps.querier,
            validated_asset_token.clone(),
            env.contract.address,
        )?;

        // Execute send on the given CW20 asset contract from the collector contract
        // to Astroport asset-UST pair contract to trigger swap on Astroport
        messages = vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: validated_asset_token.to_string(),
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
        attr("asset_token", validated_asset_token.to_string()),
    ]))
}

/// ## Description
/// Send staking Nebula token rewards to the Governance contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
pub fn distribute(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Get the current Nebula token balance of the collector contract
    let amount = query_token_balance(
        &deps.querier,
        config.nebula_token.clone(),
        env.contract.address,
    )?;

    // Send all Nebula tokens to the governance/distribution contract to trigger
    // `DepositReward` and distribute these tokens
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

/// ## Description
/// Exposes all the queries available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Config {}** Returns general contract parameters using a custom [`ConfigResponse`] structure.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
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

/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
