use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};

use crate::response::MsgInstantiateContractResponse;

use crate::state::{
    cluster_exists, deactivate_cluster, decrease_total_weight, get_cluster_data,
    increase_total_weight, read_all_weight, read_config, read_last_distributed, read_params,
    read_tmp_asset, read_tmp_cluster, read_total_weight, read_weight, record_cluster,
    remove_params, remove_weight, store_config, store_last_distributed, store_params,
    store_tmp_asset, store_tmp_cluster, store_total_weight, store_weight, Config,
};

use cluster_math::FPDecimal;

use protobuf::Message;

use nebula_protocol::cluster_factory::{
    ClusterExistsResponse, ClusterListResponse, ConfigResponse, DistributionInfoResponse,
    ExecuteMsg, InstantiateMsg, Params, QueryMsg,
};

use nebula_protocol::cluster::{
    ExecuteMsg as ClusterExecuteMsg, InstantiateMsg as ClusterInstantiateMsg,
};
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
use nebula_protocol::staking::{
    Cw20HookMsg as StakingCw20HookMsg, ExecuteMsg as StakingExecuteMsg,
};

use cw20::{Cw20ExecuteMsg, MinterResponse};
use terraswap::asset::{AssetInfo, PairInfo};
use terraswap::factory::ExecuteMsg as TerraswapFactoryExecuteMsg;
use terraswap::querier::query_pair_info;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

const NEBULA_TOKEN_WEIGHT: u32 = 300u32;
const NORMAL_TOKEN_WEIGHT: u32 = 30u32;

// lowering these to 1s for testing purposes
// change them back before we release anything...
// real value is 60u64
const DISTRIBUTION_INTERVAL: u64 = 1u64;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    store_config(
        deps.storage,
        &Config {
            owner: String::default(),
            nebula_token: String::default(),
            terraswap_factory: String::default(),
            staking_contract: String::default(),
            commission_collector: String::default(),
            protocol_fee_rate: msg.protocol_fee_rate,
            token_code_id: msg.token_code_id,
            cluster_code_id: msg.cluster_code_id,
            base_denom: msg.base_denom,
            genesis_time: env.block.time.seconds(),
            distribution_schedule: msg.distribution_schedule,
        },
    )?;

    store_total_weight(deps.storage, 0u32)?;
    store_last_distributed(deps.storage, env.block.time.seconds())?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::PostInitialize {
            owner,
            nebula_token,
            terraswap_factory,
            staking_contract,
            commission_collector,
        } => post_initialize(
            deps,
            env,
            owner,
            nebula_token,
            terraswap_factory,
            staking_contract,
            commission_collector,
        ),
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            cluster_code_id,
            distribution_schedule,
        } => update_config(
            deps,
            info,
            owner,
            token_code_id,
            cluster_code_id,
            distribution_schedule,
        ),
        ExecuteMsg::UpdateWeight {
            asset_token,
            weight,
        } => update_weight(deps, info, asset_token, weight),
        ExecuteMsg::CreateCluster { params } => create_cluster(deps, env, info, params),
        ExecuteMsg::Distribute {} => distribute(deps, env),
        ExecuteMsg::PassCommand { contract_addr, msg } => {
            pass_command(deps, info, contract_addr, msg)
        }
        ExecuteMsg::DecommissionCluster {
            cluster_contract,
            cluster_token,
        } => decommission_cluster(deps, info, cluster_contract, cluster_token),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn post_initialize(
    deps: DepsMut,
    env: Env,
    owner: String,
    nebula_token: String,
    terraswap_factory: String,
    staking_contract: String,
    commission_collector: String,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != String::default() {
        return Err(StdError::generic_err("unauthorized"));
    }

    config.owner = owner;
    config.nebula_token = nebula_token;
    config.terraswap_factory = terraswap_factory;
    config.staking_contract = staking_contract;
    config.commission_collector = commission_collector;
    store_config(deps.storage, &config)?;

    store_weight(deps.storage, &config.nebula_token, NEBULA_TOKEN_WEIGHT)?;
    increase_total_weight(deps.storage, NEBULA_TOKEN_WEIGHT)?;

    let neb_addr = config.nebula_token;

    terraswap_creation_hook(deps, env, neb_addr)
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    token_code_id: Option<u64>,
    cluster_code_id: Option<u64>,
    distribution_schedule: Option<Vec<(u64, u64, Uint128)>>,
) -> StdResult<Response> {
    let mut config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    if let Some(distribution_schedule) = distribution_schedule {
        config.distribution_schedule = distribution_schedule;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(cluster_code_id) = cluster_code_id {
        config.cluster_code_id = cluster_code_id;
    }

    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

pub fn update_weight(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    weight: u32,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    let origin_weight = read_weight(deps.storage, &asset_token)?;
    store_weight(deps.storage, &asset_token, weight)?;

    let origin_total_weight = read_total_weight(deps.storage)?;
    store_total_weight(deps.storage, origin_total_weight + weight - origin_weight)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_weight"),
        attr("asset_token", asset_token),
        attr("weight", weight.to_string()),
    ]))
}

// just for by passing command to other contract like update config
pub fn pass_command(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    msg: Binary,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    Ok(
        Response::new().add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds: vec![],
        })]),
    )
}

/*
Whitelisting process
1. Create asset token contract with `config.token_code_id` with `minter` argument
2. Call `TokenCreationHook`
    2-1. Initialize distribution info
    2-2. Register asset and oracle feeder to oracle contract
    2-3. Create terraswap pair through terraswap factory
3. Call `TerraswapCreationHook`
    3-1. Register asset to staking contract
*/
pub fn create_cluster(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: Params,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    if read_params(deps.storage).is_ok() {
        return Err(StdError::generic_err(
            "A cluster registration process is in progress",
        ));
    }

    store_params(deps.storage, &params)?;

    Ok(Response::new()
        .add_submessages(vec![SubMsg {
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: config.cluster_code_id,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&ClusterInstantiateMsg {
                    name: params.name.clone(),
                    description: params.description.clone(),
                    owner: env.contract.address.to_string(),
                    pricing_oracle: params.pricing_oracle.clone(),
                    target_oracle: params.target_oracle.clone(),
                    penalty: params.penalty,
                    factory: env.contract.address.to_string(),
                    cluster_token: None,
                    target: params.target,
                })?,
            }
            .into(),
            gas_limit: None,
            id: 1,
            reply_on: ReplyOn::Success,
        }])
        .add_attributes(vec![
            attr("action", "create_cluster"),
            attr("symbol", params.symbol.clone()),
            attr("name", params.name),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        1 => {
            // get new token's contract address
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                msg.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
            })?;
            let asset_token = res.get_contract_address();

            token_creation_hook(deps, env, asset_token.to_string())
        }
        2 => {
            let cluster = read_tmp_cluster(deps.storage)?;

            // get new token's contract address
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                msg.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
            })?;
            let token = res.get_contract_address();

            set_cluster_token_hook(deps, env, cluster, token.to_string())
        }
        3 => {
            let token = read_tmp_asset(deps.storage)?;
            terraswap_creation_hook(deps, env, token)
        }
        _ => Err(StdError::generic_err("reply id is invalid")),
    }
}

/*
TokenCreationHook
1. Initialize distribution info
2. Register asset and oracle feeder to oracle contract
3. Create terraswap pair through terraswap factory with `TerraswapCreationHook`
*/
pub fn token_creation_hook(deps: DepsMut, _env: Env, cluster: String) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    // If the param is not exists, it means there is no cluster registration process in progress
    let params: Params = match read_params(deps.storage) {
        Ok(v) => v,
        Err(_) => {
            return Err(StdError::generic_err(
                "No cluster registration process in progress",
            ))
        }
    };

    let penalty = params.penalty;

    record_cluster(deps.storage, &cluster)?;
    store_tmp_cluster(deps.storage, &cluster)?;
    Ok(Response::new()
        .add_messages(vec![
            // tell penalty contract to set owner to cluster
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: penalty,
                funds: vec![],
                msg: to_binary(&PenaltyExecuteMsg::UpdateConfig {
                    owner: Some(cluster.clone()),
                    penalty_params: None,
                })?,
            }),
            // Set cluster owner (should end up being governance)
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster.clone(),
                funds: vec![],
                msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
                    owner: Some(config.owner),
                    name: None,
                    description: None,
                    cluster_token: None,
                    pricing_oracle: None,
                    target_oracle: None,
                    penalty: None,
                    target: None,
                })?,
            }),
        ])
        .add_submessages(vec![SubMsg {
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: config.token_code_id,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&TokenInstantiateMsg {
                    name: params.name.clone(),
                    symbol: params.symbol,
                    decimals: 6u8,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: cluster.clone(),
                        cap: None,
                    }),
                })?,
            }
            .into(),
            gas_limit: None,
            id: 2,
            reply_on: ReplyOn::Success,
        }])
        .add_attributes(vec![attr("cluster_addr", cluster.as_str())]))
}

/// Set Token Hook
pub fn set_cluster_token_hook(
    deps: DepsMut,
    _env: Env,
    cluster: String,
    token: String,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;

    // If the param is not exists, it means there is no cluster registration process in progress
    let params: Params = match read_params(deps.storage) {
        Ok(v) => v,
        Err(_) => {
            return Err(StdError::generic_err(
                "No cluster registration process in progress",
            ))
        }
    };

    // If weight is given as params, we use that or just use default
    let weight = if let Some(weight) = params.weight {
        weight
    } else {
        NORMAL_TOKEN_WEIGHT
    };

    store_weight(deps.storage, &token, weight)?;
    increase_total_weight(deps.storage, weight)?;

    // Remove params == clear flag
    remove_params(deps.storage);
    store_tmp_asset(deps.storage, &token)?;
    Ok(Response::new()
        .add_messages(vec![
            //Set cluster token
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster.clone(),
                funds: vec![],
                msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
                    owner: None,
                    name: None,
                    description: None,
                    cluster_token: Some(token.clone()),
                    pricing_oracle: None,
                    target_oracle: None,
                    penalty: None,
                    target: None,
                })?,
            }),
        ])
        .add_submessages(vec![SubMsg {
            // set up terraswap pair
            msg: WasmMsg::Execute {
                contract_addr: config.terraswap_factory,
                funds: vec![],
                msg: to_binary(&TerraswapFactoryExecuteMsg::CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: config.base_denom,
                        },
                        AssetInfo::Token {
                            contract_addr: token.clone(),
                        },
                    ],
                })?,
            }
            .into(),
            gas_limit: None,
            id: 3,
            reply_on: ReplyOn::Success,
        }])
        .add_attributes(vec![
            attr("action", "set_cluster_token"),
            attr("cluster", cluster),
            attr("token", token),
        ]))
}
/// 1. Register asset and liquidity(LP) token to staking contract
pub fn terraswap_creation_hook(
    deps: DepsMut,
    _env: Env,
    asset_token: String,
) -> StdResult<Response> {
    // Now terraswap contract is already created,
    // and liquidty token also created
    let config: Config = read_config(deps.storage)?;

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: asset_token.clone(),
        },
    ];

    // Load terraswap pair info
    let pair_info: PairInfo = query_pair_info(
        &deps.querier,
        Addr::unchecked(config.terraswap_factory),
        &asset_infos,
    )?;

    // Execute staking contract to register staking token of newly created asset
    Ok(
        Response::new().add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.staking_contract,
            funds: vec![],
            msg: to_binary(&StakingExecuteMsg::RegisterAsset {
                asset_token,
                staking_token: pair_info.liquidity_token,
            })?,
        })]),
    )
}

/*
Distribute
Anyone can execute distribute operation to distribute
nebula inflation rewards on the staking pool
*/
pub fn distribute(deps: DepsMut, env: Env) -> StdResult<Response> {
    let last_distributed = read_last_distributed(deps.storage)?;
    if last_distributed + DISTRIBUTION_INTERVAL > env.block.time.seconds() {
        return Err(StdError::generic_err(
            "Cannot distribute nebula token before interval",
        ));
    }

    let config: Config = read_config(deps.storage)?;
    let time_elapsed = env.block.time.seconds() - config.genesis_time;
    let last_time_elapsed = last_distributed - config.genesis_time;
    let mut target_distribution_amount: Uint128 = Uint128::zero();
    for s in config.distribution_schedule.iter() {
        if s.0 > time_elapsed || s.1 < last_time_elapsed {
            continue;
        }

        // min(s.1, time_elapsed) - max(s.0, last_time_elapsed)
        let time_duration =
            std::cmp::min(s.1, time_elapsed) - std::cmp::max(s.0, last_time_elapsed);

        let time_slot = s.1 - s.0;
        let distribution_amount_per_sec: Decimal = Decimal::from_ratio(s.2, time_slot);
        target_distribution_amount +=
            distribution_amount_per_sec * Uint128::new(time_duration as u128);
    }

    let staking_contract = config.staking_contract;
    let nebula_token = config.nebula_token;

    let (rewards, distribution_amount) =
        _compute_rewards(deps.as_ref(), target_distribution_amount)?;

    // store last distributed
    store_last_distributed(deps.storage, env.block.time.seconds())?;
    // mint token to self and try send minted tokens to staking contract

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nebula_token,
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: staking_contract,
                amount: distribution_amount,
                msg: to_binary(&StakingCw20HookMsg::DepositReward { rewards })?,
            })?,
            funds: vec![],
        })])
        .add_attributes(vec![
            attr("action", "distribute"),
            attr("distribution_amount", distribution_amount.to_string()),
        ]))
}

pub fn _compute_rewards(
    deps: Deps,
    target_distribution_amount: Uint128,
) -> StdResult<(Vec<(String, Uint128)>, Uint128)> {
    let total_weight: u32 = read_total_weight(deps.storage)?;
    let mut distribution_amount: FPDecimal = FPDecimal::zero();
    let weights: Vec<(String, u32)> = read_all_weight(deps.storage)?;
    let rewards: Vec<(String, Uint128)> = weights
        .iter()
        .map(|w| {
            let mut amount =
                FPDecimal::from(target_distribution_amount.u128()) * FPDecimal::from(w.1 as u128);
            if amount == FPDecimal::zero() {
                return Err(StdError::generic_err("cannot distribute zero amount"));
            }
            amount = amount / FPDecimal::from(total_weight as u128);
            distribution_amount = distribution_amount + amount;
            Ok((w.0.clone(), Uint128::new(u128::from(amount))))
        })
        .filter(|m| m.is_ok())
        .collect::<StdResult<Vec<(String, Uint128)>>>()?;
    Ok((rewards, Uint128::new(u128::from(distribution_amount))))
}

pub fn decommission_cluster(
    deps: DepsMut,
    info: MessageInfo,
    cluster_contract: String,
    cluster_token: String,
) -> StdResult<Response> {
    let config: Config = read_config(deps.storage)?;
    if config.owner != info.sender.to_string() {
        return Err(StdError::generic_err("unauthorized"));
    }

    let weight = read_weight(deps.storage, &cluster_token.clone())?;
    remove_weight(deps.storage, &cluster_token.clone());
    decrease_total_weight(deps.storage, weight)?;
    deactivate_cluster(deps.storage, &cluster_contract)?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_contract.clone(),
            funds: vec![],
            msg: to_binary(&ClusterExecuteMsg::Decommission {})?,
        })])
        .add_attributes(vec![
            attr("action", "decommission_asset"),
            attr("cluster_token", cluster_token.to_string()),
            attr("cluster_contract", cluster_contract),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ClusterExists { contract_addr } => {
            to_binary(&query_cluster_exists(deps, contract_addr)?)
        }
        QueryMsg::ClusterList {} => to_binary(&query_clusters(deps)?),
        QueryMsg::DistributionInfo {} => to_binary(&query_distribution_info(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner,
        nebula_token: state.nebula_token,
        terraswap_factory: state.terraswap_factory,
        staking_contract: state.staking_contract,
        commission_collector: state.commission_collector,
        protocol_fee_rate: state.protocol_fee_rate,
        token_code_id: state.token_code_id,
        cluster_code_id: state.cluster_code_id,
        base_denom: state.base_denom,
        genesis_time: state.genesis_time,
        distribution_schedule: state.distribution_schedule,
    };

    Ok(resp)
}

pub fn query_distribution_info(deps: Deps) -> StdResult<DistributionInfoResponse> {
    let weights: Vec<(String, u32)> = read_all_weight(deps.storage)?;
    let last_distributed = read_last_distributed(deps.storage)?;
    let resp = DistributionInfoResponse {
        last_distributed,
        weights: weights
            .iter()
            .map(|w| Ok((w.0.clone(), w.1)))
            .collect::<StdResult<Vec<(String, u32)>>>()?,
    };

    Ok(resp)
}

pub fn query_cluster_exists(
    deps: Deps,
    cluster_address: String,
) -> StdResult<ClusterExistsResponse> {
    Ok(ClusterExistsResponse {
        exists: cluster_exists(deps.storage, &cluster_address)?,
    })
}

pub fn query_clusters(deps: Deps) -> StdResult<ClusterListResponse> {
    Ok(ClusterListResponse {
        contract_infos: get_cluster_data(deps.storage)?,
    })
}
