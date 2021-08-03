use crate::contract::{handle, init, query};
use crate::querier::load_token_balance;

use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, log, to_binary, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HumanAddr, StdError, Uint128, WasmMsg,
};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use nebula_protocol::incentives_custody::{HandleMsg, InitMsg, QueryMsg};

const OWNER: &str = "owner0000";
const NEB_TOKEN: &str = "nebula_token0000";

/// Convenience function for creating inline HumanAddr
pub fn h(s: &str) -> HumanAddr {
    HumanAddr(s.to_string())
}

#[test]
fn proper_initialization() {
    let msg = InitMsg {
        owner: h(OWNER),
        neb_token: h(NEB_TOKEN),
    };

    let env = mock_env(OWNER, &[]);
    let mut deps = mock_dependencies(20, &[]);
    let res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
    assert_eq!(0, res.messages.len());
}