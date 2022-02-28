#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    attr, to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::state::{
    read_claimed, read_config, read_latest_stage, read_merkle_root, store_claimed, store_config,
    store_latest_stage, store_merkle_root, Config,
};
use nebula_protocol::airdrop::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, LatestStageResponse,
    MerkleRootResponse, QueryMsg,
};

use cw20::Cw20ExecuteMsg;
use sha3::Digest;
use std::convert::TryInto;

/// ## Description
/// Creates a new contract with the specified parameters in the [`InstantiateMsg`].
/// Returns the [`Response`] with the specified attributes if the operation was successful, or a [`ContractError`] if
/// the contract was not created.
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_info** is an object of type [`MessageInfo`].
///
/// - **msg** is a message of type [`InstantiateMsg`] which contains the basic settings for creating a contract.
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
            owner: deps.api.addr_validate(msg.owner.as_str())?,
            nebula_token: deps.api.addr_validate(msg.nebula_token.as_str())?,
        },
    )?;

    let stage: u8 = 0;
    store_latest_stage(deps.storage, stage)?;

    Ok(Response::default())
}

/// ## Description
/// Exposes all the execute functions available in the contract.
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **msg** is an object of type [`ExecuteMsg`].
///
/// ## Commands
/// - **ExecuteMsg::UpdateConfig {
///             owner,
///             nebula_token,
///         }** Updates general contract parameters.
///
/// - **ExecuteMsg::RegisterMerkleRoot {
///             merkle_root,
///         }** Registers a new merkle root.
///
/// - **ExecuteMsg::Claim {
///             stage,
///             amount,
///             proof,
///         }** Claims rewards of the msg sender.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            nebula_token,
        } => update_config(deps, info, owner, nebula_token),
        ExecuteMsg::RegisterMerkleRoot { merkle_root } => {
            register_merkle_root(deps, info, merkle_root)
        }
        ExecuteMsg::Claim {
            stage,
            amount,
            proof,
        } => claim(deps, info, stage, amount, proof),
    }
}

/// ## Description
/// Updates general contract configurations. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **owner** is an object of type [`Option<String>`] which is a new owner address to update.
///
/// - **nebula_token** is an object of type [`Option<String>`] which is the address of
///     the new Nebula token contract.
///
/// ##Executor
/// Only the owner can execute this.
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    nebula_token: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = read_config(deps.storage)?;

    // Permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // Validate address format
        config.owner = deps.api.addr_validate(owner.as_str())?;
    }
    if let Some(nebula_token) = nebula_token {
        // Validate address format
        config.nebula_token = deps.api.addr_validate(nebula_token.as_str())?;
    }

    store_config(deps.storage, &config)?;
    Ok(Response::new().add_attributes(vec![attr("action", "update_config")]))
}

/// ## Description
/// Checks if `merkle_root` is a valid hex string of byte32. Otherwise returns [`ContractError`].
/// ## Params
/// - **merkle_root** is an object of type [`String`]
fn validate_merkle_root(merkle_root: String) -> Result<(), ContractError> {
    let mut root_buf: [u8; 32] = [0; 32];
    match hex::decode_to_slice(merkle_root, &mut root_buf) {
        Ok(_) => Ok(()),
        Err(_) => Err(ContractError::InvalidMerkle {}),
    }
}

/// ## Description
/// Registers a new merkle root under a next stage. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **merkle_root** is an object of type [`String`] which is a new merkle root to register.
///
/// ##Executor
/// Only the owner can execute this.
pub fn register_merkle_root(
    deps: DepsMut,
    info: MessageInfo,
    merkle_root: String,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;

    // Permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Validate `merkle_root` string
    validate_merkle_root(merkle_root.clone())?;

    // Store the validated `merkle_root` under a new stage and update the latest stage
    let latest_stage: u8 = read_latest_stage(deps.storage)?;
    let stage = latest_stage + 1;

    store_merkle_root(deps.storage, stage, merkle_root.to_string())?;
    store_latest_stage(deps.storage, stage)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("stage", stage.to_string()),
        attr("merkle_root", merkle_root),
    ]))
}

/// ## Description
/// Claims airdrop for the message sender. Returns a [`ContractError`] on failure.
///
/// ## Params
/// - **deps** is an object of type [`DepsMut`].
///
/// - **info** is an object of type [`MessageInfo`].
///
/// - **stage** is an object of type [`u8`] which is a stage of airdrop to be claimed.
///
/// - **amount** is an object of type [`Uint128`] which is the amount of the airdrop
///     for the sender at the specified stage.
///
/// - **proof** is an object of type [`Vec<String>`] which is a merkle proof
///     at the specified stage.
pub fn claim(
    deps: DepsMut,
    info: MessageInfo,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    let config: Config = read_config(deps.storage)?;
    // Read the merkle root stored at the specified stage
    let merkle_root: String = read_merkle_root(deps.storage, stage)?;

    // If user claimed target stage, return err
    if read_claimed(deps.storage, &info.sender, stage)? {
        return Err(ContractError::AlreadyClaimed {});
    }

    // Compute a merkle leaf hash from sender address and the given airdrop amount
    let user_input: String = info.sender.to_string() + &amount.to_string();
    let mut hash: [u8; 32] = sha3::Keccak256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .expect("Wrong length");

    // Compute a merkle root from the merkle leaf and provided proof
    for p in proof {
        let mut proof_buf: [u8; 32] = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf).unwrap();
        hash = if bytes_cmp(hash, proof_buf) == std::cmp::Ordering::Less {
            sha3::Keccak256::digest(&[hash, proof_buf].concat())
                .as_slice()
                .try_into()
                .expect("Wrong length")
        } else {
            sha3::Keccak256::digest(&[proof_buf, hash].concat())
                .as_slice()
                .try_into()
                .expect("Wrong length")
        };
    }

    // Validate if the computed merkle root matches the stored merkle root
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf).unwrap();
    if root_buf != hash {
        return Err(ContractError::MerkleVerification {});
    }

    // Update claim index to the current stage
    store_claimed(deps.storage, &info.sender, stage)?;

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            })?,
        }))
        .add_attributes(vec![
            attr("action", "claim"),
            attr("stage", stage.to_string()),
            attr("address", info.sender.to_string()),
            attr("amount", amount.to_string()),
        ]))
}

/// ## Description
/// Compare two arrays of byte32.
/// ## Params
/// - **a** is an array with size 32 of type [`u8`].
///
/// - **b** is an array with size 32 of type [`u8`].
fn bytes_cmp(a: [u8; 32], b: [u8; 32]) -> std::cmp::Ordering {
    let mut i = 0;
    while i < 32 {
        match a[i].cmp(&b[i]) {
            std::cmp::Ordering::Greater => {
                return std::cmp::Ordering::Greater;
            }
            std::cmp::Ordering::Less => {
                return std::cmp::Ordering::Less;
            }
            std::cmp::Ordering::Equal => i += 1,
        }
    }

    std::cmp::Ordering::Equal
}

/// ## Description
/// Exposes all the queries available in the contract.
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **msg** is an object of type [`QueryMsg`].
///
/// ## Commands
/// - **QueryMsg::Config {}** Returns general contract parameters using a custom [`ConfigResponse`] structure.
///
/// - **QueryMsg::MerkleRoot {
///             stage,
///         }** Returns the registered merkle root of a specific stage.
///
/// - **QueryMsg::LatestStage {}** Returns the latest stage that has a registered merkle root.
///
/// - **QueryMsg::IsClaimed {
///             stage,
///             address,
///         }** Return whether the specified address already claims airdrop at the given stage or not.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
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
    };

    Ok(resp)
}

/// ## Description
/// Returns the merkle root registered under the given stage.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **stage** is an object of type [`u8`].
pub fn query_merkle_root(deps: Deps, stage: u8) -> StdResult<MerkleRootResponse> {
    let merkle_root = read_merkle_root(deps.storage, stage)?;
    let resp = MerkleRootResponse { stage, merkle_root };

    Ok(resp)
}

/// ## Description
/// Returns the latest stage containing a merkle root in the airdrop contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = read_latest_stage(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

/// ## Description
/// Returns whether the specified address already claimed their airdrop of the given stage.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **stage** is an object of type [`u8`].
///
/// - **address* is an object of type [`String`].
pub fn query_is_claimed(deps: Deps, stage: u8, address: String) -> StdResult<IsClaimedResponse> {
    let resp = IsClaimedResponse {
        is_claimed: read_claimed(
            deps.storage,
            &deps.api.addr_validate(address.as_str())?,
            stage,
        )?,
    };

    Ok(resp)
}
