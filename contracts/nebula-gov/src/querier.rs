use cosmwasm_std::{Addr, Binary, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmQuery};

use cosmwasm_storage::to_length_prefixed;

/// ## Description
/// Queries CW20 asset `contract_addr` balance of `account_addr`.
///
/// ## Params
/// - **querier** is a reference to an object of type [`QuerierWrapper`].
///
/// - **contract_addr** is a reference to an object of type [`Addr`] which is an address
///     of CW20 token contract.
///
/// - **account_addr** is a reference to an object of type [`Addr`] which is an address
///     of the account to be queried.
pub fn load_token_balance(
    querier: &QuerierWrapper,
    contract_addr: &Addr,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    // Load balance from the token contract
    let res: Uint128 = querier
        .query(&QueryRequest::Wasm(WasmQuery::Raw {
            contract_addr: contract_addr.to_string(),
            key: Binary::from(concat(
                &to_length_prefixed(b"balance").to_vec(),
                account_addr.as_bytes(),
            )),
        }))
        .unwrap_or_else(|_| Uint128::zero());

    Ok(res)
}

/// # Description
/// Concatenates two `u8` arrays.
///
/// ## Params
/// - **namespace** is a reference to an object of type [`u8`].
///
/// - **key** is a reference to an object of type [`u8`].
#[inline]
fn concat(namespace: &[u8], key: &[u8]) -> Vec<u8> {
    let mut k = namespace.to_vec();
    k.extend_from_slice(key);
    k
}
