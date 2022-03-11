use astroport::asset::{AssetInfo, PairInfo};
use astroport::factory::{ExecuteMsg as AstroportFactoryExecuteMsg, PairType};
use astroport::querier::query_pair_info;
use astroport::token::InstantiateMsg as TokenInstantiateMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Reply,
    ReplyOn, Response, StdResult, Storage, SubMsg, Uint128, WasmMsg,
};
use cosmwasm_std::{entry_point, StdError};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use protobuf::Message;

use nebula_protocol::cluster::{
    ExecuteMsg as ClusterExecuteMsg, InstantiateMsg as ClusterInstantiateMsg,
};
use nebula_protocol::cluster_factory::{
    ClusterExistsResponse, ClusterListResponse, ConfigResponse, DistributionInfoResponse,
    ExecuteMsg, InstantiateMsg, Params, QueryMsg,
};
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
use nebula_protocol::staking::{
    Cw20HookMsg as StakingCw20HookMsg, ExecuteMsg as StakingExecuteMsg,
};

use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    cluster_exists, deactivate_cluster, decrease_total_weight, get_cluster_data,
    increase_total_weight, read_all_weight, read_config, read_last_distributed, read_params,
    read_tmp_asset, read_tmp_cluster, read_total_weight, read_weight, record_cluster,
    remove_params, remove_weight, store_config, store_last_distributed, store_params,
    store_tmp_asset, store_tmp_cluster, store_total_weight, store_weight, Config,
};

/// Nebula reward distribution weight for Nebula staking pool.
const NEBULA_TOKEN_WEIGHT: u32 = 300u32;
/// Default Nebula reward distribution weight for cluster LP staking pool.
const NORMAL_TOKEN_WEIGHT: u32 = 30u32;

/// Nebula reward distribution interval.
const DISTRIBUTION_INTERVAL: u64 = 60u64;

/// ## Description
/// Creates a new contract with the specified parameters packed in the `msg` variable.
/// Returns a [`Response`] with the specified attributes if the operation was successful,
/// or a [`ContractError`] if the contract was not created.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg**  is a message of type [`InstantiateMsg`] which contains the parameters used for creating the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    store_config(
        deps.storage,
        &Config {
            owner: Addr::unchecked(String::default()),
            nebula_token: Addr::unchecked(String::default()),
            astroport_factory: Addr::unchecked(String::default()),
            staking_contract: Addr::unchecked(String::default()),
            commission_collector: Addr::unchecked(String::default()),
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
/// - **ExecuteMsg::PostInitialize {
///             owner,
///             nebula_token,
///             astroport_factory,
///             staking_contract,
///             commission_collector,
///         }** Adds necessary factory contract settings after the initialization.
///
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             token_code_id,
///             cluster_code_id,
///             distribution_schedule,
///         }** Updates general factory contract parameters.
///
/// - **ExecuteMsg::CreateCluster {
///             params,
///         }** Creates a new asset cluster.
///
/// - **ExecuteMsg::DecommissionCluster {
///             cluster_contract,
///             cluster_token,
///         }** Decommissions an active cluster.
///
/// - **ExecuteMsg::UpdateWeight {
///             asset_token,
///             weight,
///         }** Updates reward distribution weight of the specific cluster LP.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PostInitialize {
            owner,
            nebula_token,
            astroport_factory,
            staking_contract,
            commission_collector,
        } => post_initialize(
            deps,
            env,
            owner,
            nebula_token,
            astroport_factory,
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
        ExecuteMsg::CreateCluster { params } => create_cluster(deps, env, info, params),
        ExecuteMsg::DecommissionCluster {
            cluster_contract,
            cluster_token,
        } => decommission_cluster(deps, info, cluster_contract, cluster_token),
        ExecuteMsg::UpdateWeight {
            asset_token,
            weight,
        } => update_weight(deps, info, asset_token, weight),
        ExecuteMsg::Distribute {} => distribute(deps, env),
        ExecuteMsg::PassCommand { contract_addr, msg } => {
            pass_command(deps, info, contract_addr, msg)
        }
    }
}

/// ## Description
/// Updates the cluster factory contract settings after initilization.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **owner** is an object of type [`String`] which is an owner to update.
///
/// - **nebula_token** is an object of type [`String`] which is an address
///     of the Nebula token contract.
///
/// - **astroport_factory** is an object of type [`String`] which is an address
///     of the Astroport factory contract.
///
/// - **staking_contract** is an object of type [`String`] which is an address
///     of the LP staking contract, i.e. a contract for staking Nebula/cluster LP tokens.
///
/// - **commission_collector** is an object of type [`String`] which is an address
///     of the commission collector contract.
#[allow(clippy::too_many_arguments)]
pub fn post_initialize(
    deps: DepsMut,
    env: Env,
    owner: String,
    nebula_token: String,
    astroport_factory: String,
    staking_contract: String,
    commission_collector: String,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check - can only execute when there is no owner
    // i.e., after the initialization step
    if config.owner != Addr::unchecked(String::default()) {
        return Err(ContractError::Unauthorized {});
    }

    config.owner = deps.api.addr_validate(owner.as_str())?;
    config.nebula_token = deps.api.addr_validate(nebula_token.as_str())?;
    config.astroport_factory = deps.api.addr_validate(astroport_factory.as_str())?;
    config.staking_contract = deps.api.addr_validate(staking_contract.as_str())?;
    config.commission_collector = deps.api.addr_validate(commission_collector.as_str())?;
    store_config(deps.storage, &config)?;

    // Add Nebula staking weight for the reward distribution
    store_weight(deps.storage, &config.nebula_token, NEBULA_TOKEN_WEIGHT)?;
    increase_total_weight(deps.storage, NEBULA_TOKEN_WEIGHT)?;

    let neb_addr = config.nebula_token;

    // Create NEB-UST LP on Astroport
    astroport_creation_hook(deps, env, neb_addr)
}

/// ## Description
/// Updates general contract settings. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is an owner to update.
///
/// - **token_code_id** is an object of type [`Option<String>`] which is an ID of
///     the uploaded CW20 contract code.
///
/// - **cluster_code_id** is an object of type [`Option<String>`] which is an ID of
///     the uploaded cluster contract code.
///
/// - **distribution_schedule** is an object of type [`Option<Vec<(u64, u64, Uint128)>>`]
///     which is a distribution schedule containing tuples of distribution period,
///     [start_time, end_time, distribution_amount].
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    token_code_id: Option<u64>,
    cluster_code_id: Option<u64>,
    distribution_schedule: Option<Vec<(u64, u64, Uint128)>>,
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

/// ## Definition
/// Updates Nebula reward distribution weight of a LP staking pool (can either be
///     Nebula LP pool or a cluster LP pool).
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **asset_token** is an object of type [`String`] which is the address of the
///     Nebula token or a cluster token.
///
/// - **weight** is an object of type [`u32`] which is the distribution weight to update.
///
/// ## Executor
/// Only the owner can execute this.
pub fn update_weight(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    weight: u32,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_asset_token = deps.api.addr_validate(asset_token.as_str())?;
    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Read the current distribution weight and overwrite with the new weight
    let origin_weight = read_weight(deps.storage, &validated_asset_token)?;
    store_weight(deps.storage, &validated_asset_token, weight)?;

    // Update the total distribution weight
    let origin_total_weight = read_total_weight(deps.storage)?;
    store_total_weight(deps.storage, origin_total_weight + weight - origin_weight)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_weight"),
        attr("asset_token", asset_token),
        attr("weight", weight.to_string()),
    ]))
}

/// ## Definition
/// Passes command to other contract e.g. update config.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **contract_addr** is an object of type [`String`] which is the address
///     of the contract to pass the command to.
///
/// - **msg** is an object of type [`Binary`] which is the message
///     to be executed with the target contract.
///
/// ## Executor
/// Only the owner can execute this.
pub fn pass_command(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    msg: Binary,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(
        Response::new().add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds: vec![],
        })]),
    )
}

/// ## Description
/// Whitelisting process.
/// 1. Creates cluster contract from `config.cluster_code_id`.
/// 2. Calls `ClusterCreationHook`.
///     2-1. Record cluster address.
///     2-2. Create token contract from `config.token_code_id`.
/// 3. `ClusterTokenCreationHook`.
///     3-1. Initialize distribution info.
///     3-2. Register cluster token to cluster contract and set owner of cluster contract to gov contract.
///     3-3. Create astroport pair through astroport factory with `AstroportCreationHook`.
/// 4. Calls `AstroportCreationHook`.
///     4-1. Register asset to staking contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **params** is an object of type [`Params`] which contains necessary variables
///     for creating a new cluster.
///
/// ## Executor
/// Only the owner can execute this.
pub fn create_cluster(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    params: Params,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // If the param storage exists, it means there is a cluster registration process in progress
    if read_params(deps.storage).is_ok() {
        return Err(ContractError::Generic(
            "A cluster registration process is in progress".to_string(),
        ));
    }

    // Store the parameters for cluster creation process
    store_params(deps.storage, &params)?;

    // Execute `ClusterInstantiateMsg` submessage to create a new cluster contract
    // with submessage ID as 1 for Reply callback
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
                    pricing_oracle: params.pricing_oracle.to_string(),
                    target_oracle: params.target_oracle.to_string(),
                    penalty: params.penalty.to_string(),
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

/// ## Description
/// Extracts result from a message of type [`Reply`].
///
/// ## Params
/// - **msg** is an object of type [`Reply`].
fn get_res_msg(msg: Reply) -> Result<MsgInstantiateContractResponse, ContractError> {
    Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
        ContractError::Std(StdError::parse_err(
            "MsgInstantiateContractResponse",
            "failed to parse data",
        ))
    })
}

/// ## Description
/// Exposes all the reply callback functions available in the contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`Reply`] which is a response message returned
///     from executing a submessage.
///
/// ## Message IDs
/// - **1** Executes callback steps after creating a cluster contract.
///
/// - **2** Executes callback steps after creating a cluster token contract.
///
/// - **3** Executes callback steps after creating `cluster token`-`base denom` LP pair
///     in Astroport.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        1 => {
            // Get the new cluster's contract address
            let res: MsgInstantiateContractResponse = get_res_msg(msg)?;
            let cluster_contract = res.get_contract_address();

            // Callback steps after creating a new cluster
            cluster_creation_hook(deps, env, cluster_contract.to_string())
        }
        2 => {
            // Get the cluster address
            let cluster_contract = read_tmp_cluster(deps.storage)?;

            // Get the new cluster token's contract address
            let res: MsgInstantiateContractResponse = get_res_msg(msg)?;
            let cluster_token = res.get_contract_address();

            // Callback steps after creating a new cluster token
            cluster_token_creation_hook(
                deps,
                env,
                cluster_contract.to_string(),
                cluster_token.to_string(),
            )
        }
        3 => {
            // Get the cluster token address for querying UST-token LP pair
            let cluster_token = read_tmp_asset(deps.storage)?;

            // Callback steps after creating a new Astroport pair contract
            astroport_creation_hook(deps, env, cluster_token)
        }
        _ => Err(ContractError::Generic("reply id is invalid".to_string())),
    }
}

/// ## Description
/// ClusterCreationHook
/// 1. Record cluster address.
/// 2. Create token contract with `config.token_code_id`.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **cluster_contract** is an object of type [`String`] which is the address
///     of the cluster contract.
pub fn cluster_creation_hook(
    deps: DepsMut,
    _env: Env,
    cluster_contract: String,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;
    let config: Config = read_config(deps.storage)?;

    // If the param storage exists, it means there is a cluster registration process in progress
    let params: Params = match read_params(deps.storage) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::NoRegistrationInProgress {});
        }
    };

    // Register the new cluster contract as active
    record_cluster(deps.storage, &validated_cluster_contract)?;
    // Save cluster contract address for using after creating a cluster token contract
    store_tmp_cluster(deps.storage, &validated_cluster_contract)?;
    Ok(Response::new()
        .add_messages(vec![
            // Tell penalty contract to set owner to cluster
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: params.penalty.to_string(),
                funds: vec![],
                msg: to_binary(&PenaltyExecuteMsg::UpdateConfig {
                    owner: Some(validated_cluster_contract.to_string()),
                    penalty_params: None,
                })?,
            }),
        ])
        // Execute `TokenInstantiateMsg` submessage to create a new cluster token contract
        // with submessage ID as 2 for Reply callback
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
                        minter: validated_cluster_contract.to_string(),
                        cap: None,
                    }),
                })?,
            }
            .into(),
            gas_limit: None,
            id: 2,
            reply_on: ReplyOn::Success,
        }])
        .add_attributes(vec![attr(
            "cluster_addr",
            validated_cluster_contract.as_str(),
        )]))
}

/// ## Description
/// ClusterTokenCreationHook
/// 1. Initialize distribution info.
/// 2. Register cluster token to cluster contract and set owner of cluster contract to gov contract.
/// 3. Create astroport pair through astroport factory with `AstroportCreationHook`.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **cluster_contract** is an object of type [`String`] which is the address of
///     the cluster contract.
///
/// - **cluster_token** is an object of type [`String`] which is the address of
///     the cluster token contract.
pub fn cluster_token_creation_hook(
    deps: DepsMut,
    _env: Env,
    cluster_contract: String,
    cluster_token: String,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;
    let validated_cluster_token = deps.api.addr_validate(cluster_token.as_str())?;

    let config: Config = read_config(deps.storage)?;

    // If the param storage exists, it means there is a cluster registration process in progress
    let params: Params = match read_params(deps.storage) {
        Ok(v) => v,
        Err(_) => {
            return Err(ContractError::NoRegistrationInProgress {});
        }
    };

    // If weight is given as params, we use that or just use default
    let weight = if let Some(weight) = params.weight {
        weight
    } else {
        NORMAL_TOKEN_WEIGHT
    };

    // Register Nebula reward distribution weight for this cluster
    // and add to the total weight
    store_weight(deps.storage, &validated_cluster_token, weight)?;
    increase_total_weight(deps.storage, weight)?;

    // Clear in-progress registration flag
    remove_params(deps.storage);
    // Save address of the cluster token contract for using after Astroport pair creation
    store_tmp_asset(deps.storage, &validated_cluster_token)?;
    Ok(Response::new()
        .add_messages(vec![
            // Set the owner of the cluster contract to the governance contract,
            // and add the cluster token contract to the cluster contract config
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: validated_cluster_contract.to_string(),
                funds: vec![],
                msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
                    owner: Some(config.owner.to_string()),
                    name: None,
                    description: None,
                    cluster_token: Some(validated_cluster_token.to_string()),
                    pricing_oracle: None,
                    target_oracle: None,
                    penalty: None,
                    target: None,
                })?,
            }),
        ])
        // Execute `CreatePair` submessage to set up Astroport "UST - cluster token" pair
        // with submessage ID as 3 for Reply callback
        .add_submessages(vec![SubMsg {
            msg: WasmMsg::Execute {
                contract_addr: config.astroport_factory.to_string(),
                funds: vec![],
                msg: to_binary(&AstroportFactoryExecuteMsg::CreatePair {
                    pair_type: PairType::Xyk {},
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: config.base_denom,
                        },
                        AssetInfo::Token {
                            contract_addr: validated_cluster_token.clone(),
                        },
                    ],
                    init_params: None,
                })?,
            }
            .into(),
            gas_limit: None,
            id: 3,
            reply_on: ReplyOn::Success,
        }])
        .add_attributes(vec![
            attr("action", "set_cluster_token"),
            attr("cluster", validated_cluster_contract.to_string()),
            attr("token", validated_cluster_token.to_string()),
        ]))
}

/// ## Description
/// Register asset and liquidity provider (LP) token to LP staking contract.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **cluster_token** is an object of type [`Addr`] which is a validated address
///     of the cluster token contract.
pub fn astroport_creation_hook(
    deps: DepsMut,
    _env: Env,
    cluster_token: Addr,
) -> Result<Response, ContractError> {
    // Now Astroport pair contract is already created,
    // and liquidity token is also created
    let config: Config = read_config(deps.storage)?;

    // Create the pair asset for retrieving the pair info in Astroport
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: cluster_token.clone(),
        },
    ];

    // Load Astroport pair info
    let pair_info: PairInfo =
        query_pair_info(&deps.querier, config.astroport_factory, &asset_infos)?;

    // Execute staking contract to register staking token of newly created asset
    Ok(
        Response::new().add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.staking_contract.to_string(),
            funds: vec![],
            msg: to_binary(&StakingExecuteMsg::RegisterAsset {
                asset_token: cluster_token.to_string(),
                staking_token: pair_info.liquidity_token.to_string(),
            })?,
        })]),
    )
}

/// ## Description
/// Executes distribute operation to distribute Nebula inflation rewards on all LP staking pools
/// -- "UST - NEB" LP staking pool + "UST - cluster token" LP staking pools.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **env** is an object of type [`Env`].
pub fn distribute(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    // Check the gap since `last_distributed` time is larger than `DISTRIBUTION_INTERVAL`
    let last_distributed = read_last_distributed(deps.storage)?;
    if last_distributed + DISTRIBUTION_INTERVAL > env.block.time.seconds() {
        return Err(ContractError::Generic(
            "Cannot distribute nebula token before interval".to_string(),
        ));
    }

    let config: Config = read_config(deps.storage)?;

    // Compute the total Nebula token distribution amount for this distribution
    // 1. Get the current undistributed interval -- (`last_distributed`, current time)
    // 2. Find all overlapping distribution intervals with the current undistributed interval
    //      and calculate the reward that should be distributed from each interval
    let time_since_genesis = env.block.time.seconds() - config.genesis_time;
    let last_time_elapsed = last_distributed - config.genesis_time;
    let mut target_distribution_amount: Uint128 = Uint128::zero();
    for s in config.distribution_schedule.iter() {
        // Skip if not overlapping
        if s.0 > time_since_genesis || s.1 < last_time_elapsed {
            continue;
        }

        // Calculate the overlapping duration
        // min(s.1, time_elapsed) - max(s.0, last_time_elapsed)
        let time_duration =
            std::cmp::min(s.1, time_since_genesis) - std::cmp::max(s.0, last_time_elapsed);

        // Get the distributing reward from this interval
        let time_slot = s.1 - s.0;
        let distribution_amount_per_sec: Decimal = Decimal::from_ratio(s.2, time_slot);
        target_distribution_amount +=
            distribution_amount_per_sec * Uint128::new(time_duration as u128);
    }

    // Get the weighted rewards for LP token staking pools
    // `reward` is a vector of (cluster token address, reward amount) pairs
    let (rewards, distribution_amount) =
        _compute_rewards(deps.storage, target_distribution_amount)?;

    // Update `last_distributed` to be the current block time
    store_last_distributed(deps.storage, env.block.time.seconds())?;

    // Send Nebula token rewards to LP staking contract
    const CHUNK_SIZE: usize = 10;
    Ok(Response::new()
        .add_messages(
            rewards
                .chunks(CHUNK_SIZE)
                .map(|v| v.to_vec())
                .into_iter()
                .map(|rewards| {
                    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: config.nebula_token.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Send {
                            contract: config.staking_contract.to_string(),
                            amount: rewards.iter().map(|v| v.1.u128()).sum::<u128>().into(),
                            msg: to_binary(&StakingCw20HookMsg::DepositReward { rewards })?,
                        })?,
                        funds: vec![],
                    }))
                })
                .collect::<Result<Vec<CosmosMsg>, ContractError>>()?,
        )
        .add_attributes(vec![
            attr("action", "distribute"),
            attr("distribution_amount", distribution_amount.to_string()),
        ]))
}

/// ## Definition
/// Calculates rewards for each LP token staking pool based on the pool weight in the settings.
///
/// ## Params
/// - **storage** is a reference of an object implementing trait [`Storage`].
///
/// - **target_distribution_amount** is an object of type [`Uint128`] which is the total
///     rewards for the current distribution.
pub fn _compute_rewards(
    storage: &dyn Storage,
    target_distribution_amount: Uint128,
) -> Result<(Vec<(String, Uint128)>, Uint128), ContractError> {
    let total_weight: u32 = read_total_weight(storage)?;
    let mut distribution_amount: Uint128 = Uint128::zero();
    let weights: Vec<(Addr, u32)> = read_all_weight(storage)?;
    // Get a vector of pairs (cluster token address, reward amount)
    let rewards: Vec<(String, Uint128)> = weights
        .iter()
        .map(|w| {
            let mut amount = target_distribution_amount * Uint128::from(w.1);
            if amount == Uint128::zero() {
                return Err(ContractError::Generic(
                    "cannot distribute zero amount".to_string(),
                ));
            }
            amount = amount / Uint128::from(total_weight);
            distribution_amount = distribution_amount + amount;
            Ok((w.0.to_string(), amount))
        })
        .filter(|m| m.is_ok())
        .collect::<Result<Vec<(String, Uint128)>, ContractError>>()?;
    Ok((rewards, distribution_amount))
}

/// ## Definition
/// Decommissions an active cluster.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **cluster_contract** is an object of type [`String`] which is an address of
///     a cluster contract.
///
/// - **cluster_token** is an object of type [`String`] which is an address of
///     a cluster token contract corresponding with the cluster contract.
///
/// ## Executor
/// Only the owner can execute this.
pub fn decommission_cluster(
    deps: DepsMut,
    info: MessageInfo,
    cluster_contract: String,
    cluster_token: String,
) -> Result<Response, ContractError> {
    // Validate address format
    let validated_cluster_contract = deps.api.addr_validate(cluster_contract.as_str())?;
    let validated_cluster_token = deps.api.addr_validate(cluster_token.as_str())?;

    let config: Config = read_config(deps.storage)?;

    // Permission check
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Remove the weight of the given cluster token LP staking pool
    let weight = read_weight(deps.storage, &validated_cluster_token)?;
    remove_weight(deps.storage, &validated_cluster_token);
    decrease_total_weight(deps.storage, weight)?;

    // Deactivate the cluster
    deactivate_cluster(deps.storage, &validated_cluster_contract)?;

    Ok(Response::new()
        .add_messages(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: validated_cluster_contract.to_string(),
            funds: vec![],
            msg: to_binary(&ClusterExecuteMsg::Decommission {})?,
        })])
        .add_attributes(vec![
            attr("action", "decommission_asset"),
            attr("cluster_token", validated_cluster_token.to_string()),
            attr("cluster_contract", validated_cluster_contract.to_string()),
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
///
/// - **QueryMsg::ClusterExists { contract_addr }** Returns whether a given address is an active cluster.
///
/// - **QueryMsg::ClusterList {}** Returns the list of pairs (cluster contract address, active status).
///
/// - **QueryMsg::DistributionInfo {}** Returns last distributed time and reward distribution weights of
///         for the Nebula and cluster LP staking pools.
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

/// ## Description
/// Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = read_config(deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner.to_string(),
        nebula_token: state.nebula_token.to_string(),
        astroport_factory: state.astroport_factory.to_string(),
        staking_contract: state.staking_contract.to_string(),
        commission_collector: state.commission_collector.to_string(),
        protocol_fee_rate: state.protocol_fee_rate,
        token_code_id: state.token_code_id,
        cluster_code_id: state.cluster_code_id,
        base_denom: state.base_denom,
        genesis_time: state.genesis_time,
        distribution_schedule: state.distribution_schedule,
    };

    Ok(resp)
}

/// ## Description
/// Returns whether the given address is an active cluster contract address.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **cluster_address** is an object of type [`String`].
pub fn query_cluster_exists(
    deps: Deps,
    cluster_address: String,
) -> StdResult<ClusterExistsResponse> {
    Ok(ClusterExistsResponse {
        exists: cluster_exists(
            deps.storage,
            &deps.api.addr_validate(cluster_address.as_str())?,
        )?,
    })
}

/// ## Description
/// Returns the list of pairs (cluster contract address, cluster active status).
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_clusters(deps: Deps) -> StdResult<ClusterListResponse> {
    Ok(ClusterListResponse {
        contract_infos: get_cluster_data(deps.storage)?,
    })
}

/// ## Description
/// Returns distribution information containing
/// - The last distributed time.
/// - The list of pairs (cluster contract address, distribution weight of Nebula / cluster LP token staking pool).
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_distribution_info(deps: Deps) -> StdResult<DistributionInfoResponse> {
    let weights: Vec<(Addr, u32)> = read_all_weight(deps.storage)?;
    let last_distributed = read_last_distributed(deps.storage)?;
    let resp = DistributionInfoResponse {
        last_distributed,
        weights: weights
            .iter()
            .map(|w| Ok((w.0.to_string(), w.1)))
            .collect::<StdResult<Vec<(String, u32)>>>()?,
    };

    Ok(resp)
}
