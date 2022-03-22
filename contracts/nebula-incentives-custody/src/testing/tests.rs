use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::state::{read_neb, read_owner};
use crate::testing::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{attr, from_binary, to_binary, CosmosMsg, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::incentives_custody::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

const OWNER: &str = "owner0000";
const NEB_TOKEN: &str = "nebula_token0000";

/// Convenience function for creating inline String
pub fn h(s: &str) -> String {
    s.to_string()
}

#[test]
fn proper_initialization() {
    let msg = InstantiateMsg {
        owner: h(OWNER),
        nebula_token: h(NEB_TOKEN),
    };

    let info = mock_info(OWNER, &[]);
    let mut deps = mock_dependencies(&[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");
    assert_eq!(0, res.messages.len());

    let owner = read_owner(deps.as_mut().storage).unwrap();
    assert_eq!(owner, h(OWNER));

    let neb = read_neb(deps.as_mut().storage).unwrap();
    assert_eq!(neb, h(NEB_TOKEN));
}

#[test]
fn test_request_neb() {
    let msg = InstantiateMsg {
        owner: h(OWNER),
        nebula_token: h(NEB_TOKEN),
    };

    let info = mock_info(OWNER, &[]);
    let mut deps = mock_dependencies(&[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");

    let neb_amount = Uint128::new(1000u128);
    deps.querier.with_token_balances(&[(
        &h(NEB_TOKEN),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &neb_amount)],
    )]);
    let info = mock_info("random", &[]);
    let msg = ExecuteMsg::RequestNeb { amount: neb_amount };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info(OWNER, &[]);
    let msg = ExecuteMsg::RequestNeb {
        amount: Uint128::new(1000u128),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h(NEB_TOKEN),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: h(OWNER),
                amount: neb_amount,
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "request_neb"),
            attr("from", MOCK_CONTRACT_ADDR),
            attr("to", OWNER),
            attr("amount", "1000")
        ]
    );
}

#[test]
fn test_query_balance() {
    let msg = InstantiateMsg {
        owner: h(OWNER),
        nebula_token: h(NEB_TOKEN),
    };

    let info = mock_info(OWNER, &[]);
    let mut deps = mock_dependencies(&[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");
    let amount = Uint128::new(1000u128);
    deps.querier
        .with_token_balances(&[(&h(NEB_TOKEN), &[(&MOCK_CONTRACT_ADDR.to_string(), &amount)])]);

    let msg = QueryMsg::Balance {};

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let balance: Uint128 = from_binary(&res).unwrap();
    assert_eq!(balance, amount);
}

#[test]
fn test_query_config() {
    let msg = InstantiateMsg {
        owner: h(OWNER),
        nebula_token: h(NEB_TOKEN),
    };

    let info = mock_info(OWNER, &[]);
    let mut deps = mock_dependencies(&[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");

    let msg = QueryMsg::Config {};

    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(config, ConfigResponse { owner: h(OWNER) });
}

#[test]
fn test_update_config() {
    let msg = InstantiateMsg {
        owner: h(OWNER),
        nebula_token: h(NEB_TOKEN),
    };

    let info = mock_info(OWNER, &[]);
    let mut deps = mock_dependencies(&[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");

    let info = mock_info("random", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: h("owner0001"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info(OWNER, &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: h("owner0001"),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let owner = read_owner(deps.as_mut().storage).unwrap();
    assert_eq!(owner, h("owner0001"));

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "update_config"),
            attr("old_owner", OWNER),
            attr("new_owner", "owner0001")
        ]
    );
}
