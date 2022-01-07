use crate::contract::{execute, instantiate, query_config};
use crate::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_binary, Coin, CosmosMsg, Decimal, StdError, SubMsg, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::collector::{ConfigResponse, ExecuteMsg, InstantiateMsg};
use nebula_protocol::gov::Cw20HookMsg::DepositReward;
use astroport::asset::{Asset, AssetInfo};
use astroport::pair::{Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        astroport_factory: ("astroportfactory".to_string()),
        distribution_contract: ("gov0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        owner: ("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!("astroportfactory", config.astroport_factory.as_str());
    assert_eq!("uusd", config.base_denom.as_str());
}

#[test]
fn test_convert() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    }]);
    deps.querier.with_token_balances(&[(
        &"tokenAAPL".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100u128))],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::new(1000000u128))],
    );

    deps.querier.with_astroport_pairs(&[
        (&"uusdtokenAAPL".to_string(), &"pairAAPL".to_string()),
        (&"uusdtokennebula".to_string(), &"pairnebula".to_string()),
    ]);

    let msg = InstantiateMsg {
        astroport_factory: ("astroportfactory".to_string()),
        distribution_contract: ("gov0000".to_string()),
        nebula_token: ("tokennebula".to_string()),
        owner: ("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::Convert {
        asset_token: "tokenAAPL".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "tokenAAPL".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "pairAAPL".to_string(),
                amount: Uint128::new(100u128),
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: None,
                    belief_price: None,
                    to: None,
                })
                .unwrap()
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    let msg = ExecuteMsg::Convert {
        asset_token: "tokennebula".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // tax deduct 100 => 99
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "pairnebula".to_string(),
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    },
                    amount: Uint128::new(99u128),
                },
                max_spread: None,
                belief_price: None,
                to: None,
            })
            .unwrap(),
            funds: vec![Coin {
                amount: Uint128::new(99u128),
                denom: "uusd".to_string(),
            }],
        }))]
    );
}

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_token_balances(&[(
        &"nebula0000".to_string(),
        &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(100u128))],
    )]);

    let msg = InstantiateMsg {
        astroport_factory: ("astroportfactory".to_string()),
        distribution_contract: ("gov0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        owner: ("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let msg = ExecuteMsg::Distribute {};

    let info = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nebula0000".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "gov0000".to_string(),
                amount: Uint128::new(100u128),
                msg: to_binary(&DepositReward {}).unwrap(),
            })
            .unwrap(),
            funds: vec![],
        }))]
    )
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        astroport_factory: ("astroportfactory".to_string()),
        distribution_contract: ("gov0000".to_string()),
        nebula_token: ("nebula0000".to_string()),
        owner: ("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // update owner
    let msg = ExecuteMsg::UpdateConfig {
        astroport_factory: Some("astroportfactory1".to_string()),
        distribution_contract: Some("gov0001".to_string()),
        nebula_token: Some("nebula0001".to_string()),
        base_denom: Some("uusd1".to_string()),
        owner: Some("owner0001".to_string()),
    };

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();

    assert_eq!("astroportfactory1", config.astroport_factory.as_str());
    assert_eq!("gov0001", config.distribution_contract.as_str());
    assert_eq!("nebula0001", config.nebula_token.as_str());
    assert_eq!("uusd1", config.base_denom.as_str());
    assert_eq!("owner0001", config.owner.as_str());

    // failed unauthoirzed
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        astroport_factory: Some("astroportfactory1".to_string()),
        distribution_contract: Some("gov0001".to_string()),
        nebula_token: Some("nebula0001".to_string()),
        base_denom: Some("uusd1".to_string()),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }
}
