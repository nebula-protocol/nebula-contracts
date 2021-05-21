use cosmwasm_std::{log, to_binary, Api, CosmosMsg, Env, Extern, HandleResponse, HandleResult, HumanAddr, Querier, StdResult, Storage, Uint128, WasmMsg, QueryRequest, WasmQuery, StdError, Order, CanonicalAddr};

use crate::state::{
    pool_info_read, pool_info_store, read_config, read_current_n, read_from_pool_bucket,
    read_from_reward_bucket, rewards_read, rewards_store, store_current_n, Config, PoolInfo,
    RewardInfo,
};
use nebula_protocol::factory::{ClusterExistsResponse, QueryMsg::ClusterExists};
use nebula_protocol::staking::{RewardInfoResponse, RewardInfoResponseItem};

use cw20::Cw20HandleMsg;

// deposit_reward must be from reward token contract
pub fn deposit_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    rewards: Vec<(HumanAddr, Uint128)>,
    rewards_amount: Uint128,
) -> HandleResult {
    let n = read_current_n(&deps.storage)?;

    for (asset_token, amount) in rewards.iter() {
        let asset_token_raw: CanonicalAddr = deps.api.canonical_address(&asset_token)?;
        let mut pool_info: PoolInfo =
            read_from_pool_bucket(&pool_info_read(&deps.storage, n), &asset_token_raw);
        pool_info.reward_sum += *amount;
        pool_info_store(&mut deps.storage, n).save(asset_token_raw.as_slice(), &pool_info)?;
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "deposit_reward"),
            log("rewards_amount", rewards_amount.to_string()),
        ],
        data: None,
    })
}

pub fn record_penalty<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    reward_owner: &HumanAddr,
    asset_address: &HumanAddr,
    penalty_amount: Uint128,
) -> HandleResult {
    let n = read_current_n(&deps.storage)?;

    let cluster = env.message.sender;
    let cfg = read_config(&deps.storage)?;

    let res: ClusterExistsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.owner.clone(),
        msg: to_binary(&ClusterExists {
            contract_addr: cluster,
        })?,
    }))?;

    if !res.exists {
        return Err(StdError::unauthorized());
    }

    let reward_owner = deps.api.canonical_address(&reward_owner)?;
    let asset_address = deps.api.canonical_address(&asset_address)?;

    let reward_bucket = rewards_read(&deps.storage, &reward_owner);
    let mut reward_info = read_from_reward_bucket(&reward_bucket, &asset_address);

    before_share_change(&deps.storage, &asset_address, &mut reward_info)?;

    let pool_bucket = pool_info_read(&deps.storage, n);
    let mut pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);

    pool_info.penalty_sum += penalty_amount;
    reward_info.penalty += penalty_amount;

    rewards_store(&mut deps.storage, &reward_owner).save(asset_address.as_slice(), &reward_info)?;
    pool_info_store(&mut deps.storage, n).save(asset_address.as_slice(), &pool_info)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "record_penalty"),
            log("penalty_amount", penalty_amount.to_string()),
        ],
        data: None,
    })
}

// withdraw all rewards or single reward depending on asset_token
pub fn withdraw_reward<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> HandleResult {
    let reward_owner = deps.api.canonical_address(&env.message.sender)?;
    let reward_bucket = rewards_read(&deps.storage, &reward_owner);

    let reward_pairs = reward_bucket
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (k, v) = item?;
            Ok((CanonicalAddr::from(k), v))
        })
        .collect::<StdResult<Vec<(CanonicalAddr, RewardInfo)>>>()?;

    let mut amount = Uint128::zero();

    for reward_pair in reward_pairs {
        let (asset_token_raw, mut reward_info) = reward_pair;

        // Withdraw reward to pending reward
        before_share_change(&mut deps.storage, &asset_token_raw, &mut reward_info)?;

        amount += reward_info.pending_reward;
        reward_info.pending_reward = Uint128::zero();
        rewards_store(&mut deps.storage, &reward_owner)
            .save(asset_token_raw.as_slice(), &reward_info)?;
    }

    let config: Config = read_config(&deps.storage)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.nebula_token)?,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: env.message.sender,
                amount,
            })?,
            send: vec![],
        })],
        log: vec![log("action", "withdraw"), log("amount", amount.to_string())],
        data: None,
    })
}

pub fn increment_n<S: Storage>(storage: &mut S) -> StdResult<()> {
    let current_n = read_current_n(storage)?;
    store_current_n(storage, current_n + 1)?;
    Ok(())
}

// transform penalty into pending reward
// the penalty must be from before the current n
pub fn before_share_change<S: Storage>(
    storage: &S,
    asset_address: &CanonicalAddr,
    reward_info: &mut RewardInfo,
) -> StdResult<()> {
    let n = read_current_n(storage)?;
    if reward_info.penalty != Uint128::zero() && reward_info.n != n {
        let pool_bucket = pool_info_read(storage, reward_info.n);
        let pool_info = read_from_pool_bucket(&pool_bucket, &asset_address);
        // using integers here .. do we care if the remaining fractions of nebula stay in this contract?
        reward_info.pending_reward += Uint128(
            pool_info.reward_sum.u128() * reward_info.penalty.u128() / pool_info.penalty_sum.u128(),
        );
        reward_info.penalty = Uint128::zero();
    }
    reward_info.n = n;
    Ok(())
}
