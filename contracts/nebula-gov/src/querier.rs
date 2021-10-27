use cosmwasm_std::{to_binary, CanonicalAddr, Deps, QueryRequest, StdResult, Uint128, WasmQuery};

use cw20::{BalanceResponse, Cw20QueryMsg};

pub fn load_token_balance(
    deps: Deps,
    contract_addr: &CanonicalAddr,
    account_addr: &CanonicalAddr,
) -> StdResult<Uint128> {
    // load balance from the token contract
    let res: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: deps.api.addr_humanize(&contract_addr)?.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: deps.api.addr_humanize(&account_addr)?.to_string(),
        })?,
    }))?;

    Ok(res.balance)
}
