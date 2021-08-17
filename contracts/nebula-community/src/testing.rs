use crate::contract::{execute, instantiate, query};

use cosmwasm_std::testing::{mock_dependencies, mock_info};
use cosmwasm_std::{from_binary, to_binary, CosmosMsg, StdError, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::community::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        spend_limit: Uint128::from(1000000u128),
    };

    let env = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("owner0000", config.owner.as_str());
    assert_eq!("nebula0000", config.nebula_token.as_str());
    assert_eq!(Uint128::from(1000000u128), config.spend_limit);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        spend_limit: Uint128::from(1000000u128),
    };

    let env = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("owner0000", config.owner.as_str());
    assert_eq!("nebula0000", config.nebula_token.as_str());
    assert_eq!(Uint128::from(1000000u128), config.spend_limit);

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(("owner0001")),
        spend_limit: None,
    };
    let env = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, msg.clone());

    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), env, msg).unwrap();
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            nebula_token: "nebula0000".to_string(),
            spend_limit: Uint128::from(1000000u128),
        }
    );

    // Update spend_limit
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        spend_limit: Some(Uint128::from(2000000u128)),
    };
    let env = mock_info("owner0001", &[]);
    let _res = execute(deps.as_mut(), env, msg);
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            nebula_token: "nebula0000".to_string(),
            spend_limit: Uint128::from(2000000u128),
        }
    );
}

#[test]
fn test_spend() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        spend_limit: Uint128::from(1000000u128),
    };

    let env = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    // permission failed
    let msg = ExecuteMsg::Spend {
        recipient: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
    };

    let env = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    // failed due to spend limit
    let msg = ExecuteMsg::Spend {
        recipient: "addr0000".to_string(),
        amount: Uint128::from(2000000u128),
    };

    let env = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), env, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot spend more than spend_limit")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::Spend {
        recipient: "addr0000".to_string(),
        amount: Uint128::from(1000000u128),
    };

    let env = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nebula0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(1000000u128),
            })
            .unwrap(),
        })]
    );
}
