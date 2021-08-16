use cosmwasm_std::{
    to_binary, Deps, DepsMut, HumanAddr, QueryRequest, StdResult, Uint128, WasmQuery,
};

use cw20::{BalanceResponse, Cw20QueryMsg};

pub fn load_token_balance(
    deps: Deps,
    contract_addr: &HumanAddr,
    account_addr: &HumanAddr,
) -> StdResult<Uint128> {
    // load balance form the token contract
    let res: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: HumanAddr::from(contract_addr),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: HumanAddr::from(account_addr),
        })?,
    }))?;

    Ok(res.balance)
}
