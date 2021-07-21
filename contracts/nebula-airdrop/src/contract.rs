use cosmwasm_std::{
    log, to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr,
    InitResponse, InitResult, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use crate::state::{
    read_claimed, read_config, read_latest_stage, read_merkle_root, store_claimed, store_config,
    store_latest_stage, store_merkle_root, Config,
};
use nebula_protocol::airdrop::{
    ConfigResponse, HandleMsg, InitMsg, IsClaimedResponse, LatestStageResponse, MerkleRootResponse,
    QueryMsg,
};

use cw20::Cw20HandleMsg;
use sha3::Digest;
use std::convert::TryInto;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> InitResult {
    store_config(
        &mut deps.storage,
        &Config {
            owner: msg.owner,
            nebula_token: msg.nebula_token,
        },
    )?;

    let stage: u8 = 0;
    store_latest_stage(&mut deps.storage, stage)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    match msg {
        HandleMsg::UpdateConfig { owner } => update_config(deps, env, owner),
        HandleMsg::RegisterMerkleRoot { merkle_root } => {
            register_merkle_root(deps, env, merkle_root)
        }
        HandleMsg::Claim {
            stage,
            amount,
            proof,
        } => claim(deps, env, stage, amount, proof),
    }
}

pub fn update_config<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: Option<HumanAddr>,
) -> StdResult<HandleResponse> {
    let mut config: Config = read_config(&deps.storage)?;
    if env.message.sender != config.owner {
        return Err(StdError::unauthorized());
    }

    if let Some(owner) = owner {
        config.owner = owner;
    }

    store_config(&mut deps.storage, &config)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "update_config")],
        data: None,
    })
}

fn validate_merkle_root(merkle_root: String) -> StdResult<()> {
    let mut root_buf: [u8; 32] = [0; 32];
    match hex::decode_to_slice(merkle_root, &mut root_buf) {
        Ok(_) => Ok(()),
        Err(_) => Err(StdError::generic_err("Invalid merkle root")),
    }
}

pub fn register_merkle_root<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    merkle_root: String,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(&deps.storage)?;
    if env.message.sender != config.owner {
        return Err(StdError::unauthorized());
    }

    validate_merkle_root(merkle_root.clone())?;

    let latest_stage: u8 = read_latest_stage(&deps.storage)?;
    let stage = latest_stage + 1;

    store_merkle_root(&mut deps.storage, stage, merkle_root.to_string())?;
    store_latest_stage(&mut deps.storage, stage)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "register_merkle_root"),
            log("stage", stage),
            log("merkle_root", merkle_root),
        ],
        data: None,
    })
}

pub fn claim<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(&deps.storage)?;
    let merkle_root: String = read_merkle_root(&deps.storage, stage)?;

    // If user claimed target stage, return err
    if read_claimed(&deps.storage, &env.message.sender, stage)? {
        return Err(StdError::generic_err("Already claimed"));
    }

    let user_input: String = env.message.sender.to_string() + &amount.to_string();
    let mut hash: [u8; 32] = sha3::Keccak256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .expect("Wrong length");

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

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf).unwrap();

    if root_buf != hash {
        return Err(StdError::generic_err("Verification is failed"));
    }

    // Update claim index to the current stage
    store_claimed(&mut deps.storage, &env.message.sender, stage)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.nebula_token,
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: env.message.sender.clone(),
                amount,
            })?,
        })],
        log: vec![
            log("action", "claim"),
            log("stage", stage),
            log("address", env.message.sender),
            log("amount", amount),
        ],
        data: None,
    })
}

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

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: state.owner,
        nebula_token: state.nebula_token,
    };

    Ok(resp)
}

pub fn query_merkle_root<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    stage: u8,
) -> StdResult<MerkleRootResponse> {
    let merkle_root = read_merkle_root(&deps.storage, stage)?;
    let resp = MerkleRootResponse { stage, merkle_root };

    Ok(resp)
}

pub fn query_latest_stage<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<LatestStageResponse> {
    let latest_stage = read_latest_stage(&deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    stage: u8,
    address: HumanAddr,
) -> StdResult<IsClaimedResponse> {
    let resp = IsClaimedResponse {
        is_claimed: read_claimed(&deps.storage, &address, stage)?,
    };

    Ok(resp)
}
