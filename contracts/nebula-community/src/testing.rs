use crate::contract::{execute, instantiate, query};
use crate::mock_querier::mock_dependencies;

use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankMsg, CosmosMsg, StdError, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::community::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("owner0000", config.owner.as_str());
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!("owner0000", config.owner.as_str());

    let msg = ExecuteMsg::UpdateConfig {
        owner: "owner0001".to_string(),
    };
    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone());

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
        }
    );
}

#[test]
fn test_spend() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // permission failed
    let msg = ExecuteMsg::Spend {
        asset: Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("some_token_address"),
            },
            amount: Uint128::from(1000000u128),
        },
        recipient: "addr0000".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // successfully spend Native token
    let msg = ExecuteMsg::Spend {
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::from(1000000u128),
        },
        recipient: "addr0000".to_string(),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: coins(1000000u128, "uusd"),
        }))],
    );

    // successfully spend CW20 token
    let msg = ExecuteMsg::Spend {
        asset: Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("some_token_address_01"),
            },
            amount: Uint128::from(1000000u128),
        },
        recipient: "addr0000".to_string(),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "some_token_address_01".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "addr0000".to_string(),
                amount: Uint128::from(1000000u128),
            })
            .unwrap(),
        }))]
    );
}
