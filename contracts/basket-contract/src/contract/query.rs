use cosmwasm_std::{
    to_binary, Api, Binary, Extern, HumanAddr, Querier, StdResult, Storage, Uint128,
};

use crate::ext_query::{query_cw20_balance, query_cw20_token_supply, query_price};
use crate::msg::{
    BasketStateResponse, ConfigResponse, QueryMsg, StagedAmountResponse, TargetResponse,
};
use crate::state::{read_config, read_staged_asset, read_target};
use basket_math::FPDecimal;

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Target {} => to_binary(&query_target(deps)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::StagedAmount { account, asset } => {
            to_binary(&query_staged_amount(deps, &account, &asset)?)
        }
        QueryMsg::BasketState {
            basket_contract_address,
        } => to_binary(&query_basket_state(deps, &basket_contract_address)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let cfg = read_config(&deps.storage)?;
    Ok(ConfigResponse { config: cfg })
}

fn query_target<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<TargetResponse> {
    let target = read_target(&deps.storage)?;
    Ok(TargetResponse { target })
}

fn query_staged_amount<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    account: &HumanAddr,
    asset: &HumanAddr,
) -> StdResult<StagedAmountResponse> {
    let staged_amount = read_staged_asset(&deps.storage, account, asset)?;
    Ok(StagedAmountResponse { staged_amount })
}

pub fn query_basket_state<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    basket_contract_address: &HumanAddr,
) -> StdResult<BasketStateResponse> {
    let cfg = &read_config(&deps.storage)?;

    let penalty_params = cfg.penalty_params;

    // get supply from basket token
    let outstanding_balance_tokens =
        query_cw20_token_supply(&deps, &cfg.basket_token.ok_or_else())?;

    // get prices for each asset
    let prices = cfg
        .assets
        .iter()
        .map(|asset| query_price(&deps, &cfg.oracle, &asset))
        .collect::<StdResult<Vec<FPDecimal>>>()?;

    // get inventory
    let inv: Vec<Uint128> = cfg
        .assets
        .iter()
        .map(|asset| query_cw20_balance(&deps, &asset, basket_contract_address))
        .collect::<StdResult<Vec<Uint128>>>()?;

    let target = read_target(&deps.storage)?;

    Ok(BasketStateResponse {
        penalty_params,
        outstanding_balance_tokens,
        prices,
        inv,
        target,
    })
}
