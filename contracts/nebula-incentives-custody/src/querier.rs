use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, Uint128, WasmQuery};

use cw20::{BalanceResponse, Cw20QueryMsg};

/// ## Description
/// Loads a specific asset balance of the given contract.
///
/// ## Params
/// - **deps** is an object of type [`Deps`].
///
/// - **contract_addr** is a reference to an object of type [`Addr`] which is the
///     address of a CW20 contract.
///
/// - **account_addr** is a reference to an object of type [`Addr`] which is the
///     address of an account to be queried.
pub fn load_token_balance(
    deps: Deps,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // Load balance from the token contract
    let res: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    Ok(res.balance)
}
