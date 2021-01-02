use cosmwasm_std::{
    from_binary, log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse,
    HandleResult, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::msg::{Cw20HookMsg, HandleMsg, InitMsg, QueryMsg, TargetResponse};
use crate::penalty::{compute_penalty, compute_score};
use crate::state::{read_config, read_target, save_config, save_target, BasketConfig};

/// EXTERNAL QUERY
/// -- Queries the token_address contract for the current balance of account
pub fn load_balance(
    asset_address: &CanonicalAddr,
    account_address: &HumanAddr,
) -> StdResult<Uint128> {
    Ok(Uint128::zero())
}

/// EXTERNAL QUERY
/// -- Queries the oracle contract for the current asset price
pub fn load_price(
    oracle_address: &CanonicalAddr,
    asset_address: &CanonicalAddr,
) -> StdResult<Uint128> {
    Ok(Uint128::zero())
}
