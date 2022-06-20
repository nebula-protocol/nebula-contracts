use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, ContractVersion};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::community::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

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
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

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
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // successfully spend Native token
    let msg = ExecuteMsg::Spend {
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uasset".to_string(),
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
            amount: coins(1000000u128, "uasset"),
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

#[test]
fn test_pass_command() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // failed non-owner call
    let msg = ExecuteMsg::PassCommand {
        wasm_msg: WasmMsg::Execute {
            contract_addr: "contract0001".to_string(),
            msg: Binary::default(),
            funds: vec![],
        },
    };

    let info = mock_info("imposter0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // successfully pass command without funds
    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "contract0001".to_string(),
            funds: vec![],
            msg: Binary::default(),
        }))]
    );

    // successfully pass command with funds
    let msg = ExecuteMsg::PassCommand {
        wasm_msg: WasmMsg::Execute {
            contract_addr: "contract0001".to_string(),
            msg: Binary::default(),
            funds: coins(100000000u128, "uasset"),
        },
    };
    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "contract0001".to_string(),
            funds: coins(100000000u128, "uasset"),
            msg: Binary::default(),
        }))]
    );
}

#[test]
fn migration() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        owner: ("owner0000".to_string()),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-community".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
