use crate::contract::{handle, init, query};
use crate::mock_querier::mock_dependencies;
use crate::state::{read_neb, read_owner};

use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, log, to_binary, Binary, CosmosMsg, HumanAddr, StdError, Uint128, WasmMsg,
};
use cw20::Cw20HandleMsg;
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

    let owner = read_owner(&mut deps.storage).unwrap();
    assert_eq!(owner, h(OWNER));

    let neb = read_neb(&mut deps.storage).unwrap();
    assert_eq!(neb, h(NEB_TOKEN));
}

#[test]
fn test_request_neb() {
    let msg = InitMsg {
        owner: h(OWNER),
        neb_token: h(NEB_TOKEN),
    };

    let env = mock_env(OWNER, &[]);
    let mut deps = mock_dependencies(20, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let neb_amount = Uint128(1000u128);
    deps.querier
        .with_token_balances(&[(&h(NEB_TOKEN), &[(&h(MOCK_CONTRACT_ADDR), &neb_amount)])]);
    let env = mock_env("random", &[]);
    let msg = HandleMsg::RequestNeb { amount: neb_amount };
    let res = handle(&mut deps, env, msg);

    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let env = mock_env(OWNER, &[]);
    let msg = HandleMsg::RequestNeb {
        amount: Uint128(1000u128),
    };
    let res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h(NEB_TOKEN),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: h(OWNER),
                amount: neb_amount,
            })
            .unwrap(),
            send: vec![],
        })]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "request_neb"),
            log("from", MOCK_CONTRACT_ADDR),
            log("to", OWNER),
            log("amount", "1000")
        ]
    );
}

#[test]
fn test_query() {
    let msg = InitMsg {
        owner: h(OWNER),
        neb_token: h(NEB_TOKEN),
    };

    let env = mock_env(OWNER, &[]);
    let mut deps = mock_dependencies(20, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
    let amount = Uint128(1000u128);
    deps.querier
        .with_token_balances(&[(&h(NEB_TOKEN), &[(&h(MOCK_CONTRACT_ADDR), &amount)])]);

    let msg = QueryMsg::Balance {
        custody: h(MOCK_CONTRACT_ADDR),
    };

    let res = query(&deps, msg).unwrap();
    let balance_binary: Binary = from_binary(&res).unwrap();
    let balance: Uint128 = from_binary(&balance_binary).unwrap();
    assert_eq!(balance, amount);
}

#[test]
fn test_update_owner() {
    let msg = InitMsg {
        owner: h(OWNER),
        neb_token: h(NEB_TOKEN),
    };

    let env = mock_env(OWNER, &[]);
    let mut deps = mock_dependencies(20, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");

    let env = mock_env("random", &[]);
    let msg = HandleMsg::UpdateOwner {
        owner: h("owner0001"),
    };
    let res = handle(&mut deps, env, msg);

    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let env = mock_env(OWNER, &[]);
    let msg = HandleMsg::UpdateOwner {
        owner: h("owner0001"),
    };
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let owner = read_owner(&mut deps.storage).unwrap();
    assert_eq!(owner, h("owner0001"));

    assert_eq!(
        res.log,
        vec![
            log("action", "update_owner"),
            log("old_owner", OWNER),
            log("new_owner", "owner0001")
        ]
    );
}
