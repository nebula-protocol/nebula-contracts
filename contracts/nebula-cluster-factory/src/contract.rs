use cosmwasm_std::{
    log, to_binary, Api, Binary, CosmosMsg, Decimal, Env, Extern, HandleResponse, HandleResult,
    HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::state::{
    cluster_exists, deactivate_cluster, decrease_total_weight, get_cluster_data,
    increase_total_weight, read_all_weight, read_config, read_last_distributed, read_params,
    read_total_weight, read_weight, record_cluster, remove_params, remove_weight, store_config,
    store_last_distributed, store_params, store_total_weight, store_weight, Config,
};

use nebula_protocol::cluster_factory::{
    ClusterExistsResponse, ClusterListResponse, ConfigResponse, DistributionInfoResponse,
    HandleMsg, InitMsg, Params, QueryMsg,
};

use nebula_protocol::cluster::{HandleMsg as ClusterHandleMsg, InitMsg as ClusterInitMsg};
use nebula_protocol::penalty::HandleMsg as PenaltyHandleMsg;
use nebula_protocol::staking::{Cw20HookMsg as StakingCw20HookMsg, HandleMsg as StakingHandleMsg};

use cw20::{Cw20HandleMsg, MinterResponse};
use terraswap::asset::{AssetInfo, PairInfo};
use terraswap::factory::HandleMsg as TerraswapFactoryHandleMsg;
use terraswap::hook::InitHook;
use terraswap::querier::query_pair_info;
use terraswap::token::InitMsg as TokenInitMsg;

const NEBULA_TOKEN_WEIGHT: u32 = 300u32;
const NORMAL_TOKEN_WEIGHT: u32 = 30u32;

// lowering these to 1s for testing purposes
// change them back before we release anything...
// real value is 60u64
const DISTRIBUTION_INTERVAL: u64 = 1u64;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    store_config(
        &mut deps.storage,
        &Config {
            owner: HumanAddr::default(),
            nebula_token: HumanAddr::default(),
            terraswap_factory: HumanAddr::default(),
            staking_contract: HumanAddr::default(),
            commission_collector: HumanAddr::default(),
            protocol_fee_rate: msg.protocol_fee_rate,
            token_code_id: msg.token_code_id,
            cluster_code_id: msg.cluster_code_id,
            base_denom: msg.base_denom,
            genesis_time: env.block.time,
            distribution_schedule: msg.distribution_schedule,
        },
    )?;

    store_total_weight(&mut deps.storage, 0u32)?;
    store_last_distributed(&mut deps.storage, env.block.time)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::PostInitialize {
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
        HandleMsg::UpdateConfig {
            owner,
            token_code_id,
            cluster_code_id,
            distribution_schedule,
        } => update_config(
            deps,
            env,
            owner,
            token_code_id,
            cluster_code_id,
            distribution_schedule,
        ),
        HandleMsg::UpdateWeight {
            asset_token,
            weight,
        } => update_weight(deps, env, asset_token, weight),
        HandleMsg::CreateCluster { params } => create_cluster(deps, env, params),
        HandleMsg::TokenCreationHook {} => token_creation_hook(deps, env),
        HandleMsg::SetClusterTokenHook { cluster } => set_cluster_token_hook(deps, env, cluster),
        HandleMsg::TerraswapCreationHook { asset_token } => {
            terraswap_creation_hook(deps, env, asset_token)
        }
        HandleMsg::Distribute {} => distribute(deps, env),
        HandleMsg::PassCommand { contract_addr, msg } => {
            pass_command(deps, env, contract_addr, msg)
        }
        HandleMsg::DecommissionCluster {
            cluster_contract,
            cluster_token,
        } => decommission_cluster(deps, env, cluster_contract, cluster_token),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn post_initialize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    owner: HumanAddr,
    nebula_token: HumanAddr,
    terraswap_factory: HumanAddr,
    staking_contract: HumanAddr,
    commission_collector: HumanAddr,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if config.owner != HumanAddr::default() {
        return Err(StdError::unauthorized());
    }

    config.owner = owner;
    config.nebula_token = nebula_token;
    config.terraswap_factory = terraswap_factory;
    config.staking_contract = staking_contract;
    config.commission_collector = commission_collector;
    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse::default())
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
    token_code_id: Option<u64>,
    cluster_code_id: Option<u64>,
    distribution_schedule: Option<Vec<(u64, u64, Uint128)>>,
) -> HandleResult {
    let mut config: Config = read_config(&deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
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

    store_config(&mut deps.storage, &config)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

pub fn update_weight<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_token: HumanAddr,
    weight: u32,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    let origin_weight = read_weight(&deps.storage, &asset_token)?;
    store_weight(&mut deps.storage, &asset_token, weight)?;

    let origin_total_weight = read_total_weight(&deps.storage)?;
    store_total_weight(
        &mut deps.storage,
        origin_total_weight + weight - origin_weight,
    )?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "update_weight"),
            log("asset_token", asset_token),
            log("weight", weight),
        ],
        data: None,
    })
}

// just for by passing command to other contract like update config
pub fn pass_command<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contract_addr: HumanAddr,
    msg: Binary,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            send: vec![],
        })],
        log: vec![],
        data: None,
    })
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
pub fn create_cluster<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    params: Params,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    if config.owner != env.message.sender {
        return Err(StdError::unauthorized());
    }

    if read_params(&deps.storage).is_ok() {
        return Err(StdError::generic_err("A cluster registration process is in progress"));
    }

    store_params(&mut deps.storage, &params)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: config.cluster_code_id,
            send: vec![],
            label: None,
            msg: to_binary(&ClusterInitMsg {
                name: params.name.clone(),
                description: params.description.clone(),
                owner: env.contract.address.clone(),
                pricing_oracle: params.pricing_oracle.clone(),
                composition_oracle: params.composition_oracle.clone(),
                penalty: params.penalty,
                cluster_token: None,
                target: params.target,
                init_hook: Some(InitHook {
                    contract_addr: env.contract.address,
                    msg: to_binary(&HandleMsg::TokenCreationHook {})?,
                }),
            })?,
        })],
        log: vec![
            log("action", "create_cluster"),
            log("symbol", params.symbol.clone()),
            log("name", params.name),
        ],
        data: None,
    })
}

/*
TokenCreationHook
1. Initialize distribution info
2. Register asset and oracle feeder to oracle contract
3. Create terraswap pair through terraswap factory with `TerraswapCreationHook`
*/
pub fn token_creation_hook<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;

    // If the param is not exists, it means there is no cluster registration process in progress
    let params: Params = match read_params(&deps.storage) {
        Ok(v) => v,
        Err(_) => {
            return Err(StdError::generic_err(
                "No cluster registration process in progress",
            ))
        }
    };

    let penalty = params.penalty;

    let cluster = env.message.sender;
    record_cluster(&mut deps.storage, &cluster)?;
    Ok(HandleResponse {
        messages: vec![
            // tell penalty contract to set owner to cluster
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: penalty,
                send: vec![],
                msg: to_binary(&PenaltyHandleMsg::UpdateConfig {
                    owner: Some(cluster.clone()),
                    penalty_params: None,
                })?,
            }),
            // Instantiate token
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.token_code_id,
                send: vec![],
                label: None,
                msg: to_binary(&TokenInitMsg {
                    name: params.name.clone(),
                    symbol: params.symbol,
                    decimals: 6u8,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: cluster.clone(),
                        cap: None,
                    }),
                    // Set Cluster Token
                    init_hook: Some(InitHook {
                        contract_addr: env.contract.address,
                        msg: to_binary(&HandleMsg::SetClusterTokenHook {
                            cluster: cluster.clone(),
                        })?,
                    }),
                })?,
            }),
            // Set cluster owner (should end up being governance)
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster.clone(),
                send: vec![],
                msg: to_binary(&ClusterHandleMsg::UpdateConfig {
                    owner: Some(config.owner),
                    name: None,
                    description: None,
                    cluster_token: None,
                    pricing_oracle: None,
                    composition_oracle: None,
                    penalty: None,
                    target: None,
                })?,
            }),
        ],
        log: vec![log("cluster_addr", cluster.as_str())],
        data: None,
    })
}

/// Set Token Hook
pub fn set_cluster_token_hook<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    cluster: HumanAddr,
) -> HandleResult {
    let config: Config = read_config(&deps.storage)?;
    let token = env.message.sender;

    // If the param is not exists, it means there is no cluster registration process in progress
    let params: Params = match read_params(&deps.storage) {
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

    store_weight(&mut deps.storage, &token, weight)?;
    increase_total_weight(&mut deps.storage, weight)?;

    // Remove params == clear flag
    remove_params(&mut deps.storage);

    Ok(HandleResponse {
        messages: vec![
            //Set cluster token
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cluster.clone(),
                send: vec![],
                msg: to_binary(&ClusterHandleMsg::UpdateConfig {
                    owner: None,
                    name: None,
                    description: None,
                    cluster_token: Some(token.clone()),
                    pricing_oracle: None,
                    composition_oracle: None,
                    penalty: None,
                    target: None,
                })?,
            }),
            // set up terraswap pair
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.terraswap_factory,
                send: vec![],
                msg: to_binary(&TerraswapFactoryHandleMsg::CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: config.base_denom,
                        },
                        AssetInfo::Token {
                            contract_addr: token.clone(),
                        },
                    ],
                    init_hook: Some(InitHook {
                        msg: to_binary(&HandleMsg::TerraswapCreationHook {
                            asset_token: token.clone(),
                        })?,
                        contract_addr: env.contract.address,
                    }),
                })?,
            }),
        ],
        log: vec![
            log("action", "set_cluster_token"),
            log("cluster", cluster),
            log("token", token),
        ],
        data: None,
    })
}
/// 1. Register asset and liquidity(LP) token to staking contract
pub fn terraswap_creation_hook<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    asset_token: HumanAddr,
) -> HandleResult {
    // Now terraswap contract is already created,
    // and liquidty token also created
    let config: Config = read_config(&deps.storage)?;

    if config.nebula_token == asset_token {
        store_weight(&mut deps.storage, &asset_token, NEBULA_TOKEN_WEIGHT)?;
        increase_total_weight(&mut deps.storage, NEBULA_TOKEN_WEIGHT)?;
    } else if config.terraswap_factory != env.message.sender {
        return Err(StdError::unauthorized());
    }

    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: asset_token.clone(),
        },
    ];

    // Load terraswap pair info
    let pair_info: PairInfo = query_pair_info(&deps, &config.terraswap_factory, &asset_infos)?;

    // Execute staking contract to register staking token of newly created asset
    Ok(HandleResponse {
        // messages: vec![],
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.staking_contract,
            send: vec![],
            msg: to_binary(&StakingHandleMsg::RegisterAsset {
                asset_token,
                staking_token: pair_info.liquidity_token,
            })?,
        })],
        log: vec![],
        data: None,
    })
}

/*
Distribute
Anyone can execute distribute operation to distribute
nebula inflation rewards on the staking pool
*/
pub fn distribute<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let last_distributed = read_last_distributed(&deps.storage)?;
    if last_distributed + DISTRIBUTION_INTERVAL > env.block.time {
        return Err(StdError::generic_err(
            "Cannot distribute nebula token before interval",
        ));
    }

    let config: Config = read_config(&deps.storage)?;
    let time_elapsed = env.block.time - config.genesis_time;
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
        target_distribution_amount += distribution_amount_per_sec * Uint128(time_duration as u128);
    }

    let staking_contract = config.staking_contract;
    let nebula_token = config.nebula_token;

    let (rewards, distribution_amount) = _compute_rewards(&deps, target_distribution_amount)?;

    // store last distributed
    store_last_distributed(&mut deps.storage, env.block.time)?;
    // mint token to self and try send minted tokens to staking contract
    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nebula_token,
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: staking_contract,
                amount: distribution_amount,
                msg: Some(to_binary(&StakingCw20HookMsg::DepositReward { rewards })?),
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "distribute"),
            log("distribution_amount", distribution_amount.to_string()),
        ],
        data: None,
    })
}

pub fn _compute_rewards<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    target_distribution_amount: Uint128,
) -> StdResult<(Vec<(HumanAddr, Uint128)>, Uint128)> {
    let total_weight: u32 = read_total_weight(&deps.storage)?;
    let mut distribution_amount: Uint128 = Uint128::zero();
    let weights: Vec<(HumanAddr, u32)> = read_all_weight(&deps.storage)?;
    let rewards: Vec<(HumanAddr, Uint128)> = weights
        .iter()
        .map(|w| {
            let amount =
                target_distribution_amount * Decimal::from_ratio(w.1 as u128, total_weight as u128);

            if amount.is_zero() {
                return Err(StdError::generic_err("cannot distribute zero amount"));
            }
            distribution_amount += amount;
            Ok((w.0.clone(), amount))
        })
        .filter(|m| m.is_ok())
        .collect::<StdResult<Vec<(HumanAddr, Uint128)>>>()?;
    Ok((rewards, distribution_amount))
}

pub fn decommission_cluster<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    cluster_contract: HumanAddr,
    cluster_token: HumanAddr,
) -> HandleResult {
    let weight = read_weight(&deps.storage, &cluster_token.clone())?;
    remove_weight(&mut deps.storage, &cluster_token.clone());
    decrease_total_weight(&mut deps.storage, weight)?;
    deactivate_cluster(&mut deps.storage, &cluster_contract)?;

    Ok(HandleResponse {
        /// send message to set active asset
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cluster_contract.clone(),
            send: vec![],
            msg: to_binary(&ClusterHandleMsg::Decommission {})?,
        })],
        log: vec![
            log("action", "decommission_asset"),
            log("cluster_token", cluster_token.to_string()),
            log("cluster_contract", cluster_contract),
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
        QueryMsg::ClusterExists { contract_addr } => {
            to_binary(&query_cluster_exists(deps, contract_addr)?)
        }
        QueryMsg::ClusterList {} => to_binary(&query_clusters(deps)?),
        QueryMsg::DistributionInfo {} => to_binary(&query_distribution_info(deps)?),
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
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

pub fn query_distribution_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<DistributionInfoResponse> {
    let weights: Vec<(HumanAddr, u32)> = read_all_weight(&deps.storage)?;
    let last_distributed = read_last_distributed(&deps.storage)?;
    let resp = DistributionInfoResponse {
        last_distributed,
        weights: weights
            .iter()
            .map(|w| Ok((w.0.clone(), w.1)))
            .collect::<StdResult<Vec<(HumanAddr, u32)>>>()?,
    };

    Ok(resp)
}

pub fn query_cluster_exists<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    cluster_address: HumanAddr,
) -> StdResult<ClusterExistsResponse> {
    Ok(ClusterExistsResponse {
        exists: cluster_exists(&deps.storage, &cluster_address)?,
    })
}

pub fn query_clusters<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ClusterListResponse> {
    Ok(ClusterListResponse {
        contract_infos: get_cluster_data(&deps.storage)?,
    })
}
