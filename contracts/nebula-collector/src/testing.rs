use crate::contract::{execute, init, query_config};
use crate::mock_querier::mock_dependencies;
use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_binary, Coin, CosmosMsg, Decimal, HumanAddr, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::collector::{ConfigResponse, ExecuteMsg, InstantiateMsg};
use nebula_protocol::gov::Cw20HookMsg::DepositReward;
use terraswap::asset::{Asset, AssetInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, ExecuteMsg as TerraswapExecuteMsg};

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InstantiateMsg {
        terraswap_factory: HumanAddr("terraswapfactory".to_string()),
        distribution_contract: HumanAddr("gov0000".to_string()),
        nebula_token: HumanAddr("nebula0000".to_string()),
        owner: HumanAddr("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let env = mock_info("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!("terraswapfactory", config.terraswap_factory.as_str());
    assert_eq!("uusd", config.base_denom.as_str());
}

#[test]
fn test_convert() {
    let mut deps = mock_dependencies(
        20,
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(100u128),
        }],
    );
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("tokenAPPL"),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128::new(100u128))],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::new(1000000u128))],
    );

    deps.querier.with_terraswap_pairs(&[
        (&"uusdtokenAPPL".to_string(), &HumanAddr::from("pairAPPL")),
        (
            &"uusdtokennebula".to_string(),
            &HumanAddr::from("pairnebula"),
        ),
    ]);

    let msg = InstantiateMsg {
        terraswap_factory: HumanAddr("terraswapfactory".to_string()),
        distribution_contract: HumanAddr("gov0000".to_string()),
        nebula_token: HumanAddr("tokennebula".to_string()),
        owner: HumanAddr("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let env = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    let msg = ExecuteMsg::Convert {
        asset_token: HumanAddr::from("tokenAPPL"),
    };

    let env = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("tokenAPPL"),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: HumanAddr::from("pairAPPL"),
                amount: Uint128::new(100u128),
                msg: Some(
                    to_binary(&TerraswapCw20HookMsg::Swap {
                        max_spread: None,
                        belief_price: None,
                        to: None,
                    })
                    .unwrap()
                ),
            })
            .unwrap(),
            funds: vec![],
        })]
    );

    let msg = ExecuteMsg::Convert {
        asset_token: HumanAddr::from("tokennebula"),
    };

    let env = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, msg).unwrap();

    // tax deduct 100 => 99
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("pairnebula"),
            msg: to_binary(&TerraswapExecuteMsg::Swap {
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
            send: vec![Coin {
                amount: Uint128::new(99u128),
                denom: "uusd".to_string(),
            }],
        })]
    );
}

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_token_balances(&[(
        &HumanAddr::from("nebula0000"),
        &[(&HumanAddr::from(MOCK_CONTRACT_ADDR), &Uint128::new(100u128))],
    )]);

    let msg = InstantiateMsg {
        terraswap_factory: HumanAddr("terraswapfactory".to_string()),
        distribution_contract: HumanAddr("gov0000".to_string()),
        nebula_token: HumanAddr("nebula0000".to_string()),
        owner: HumanAddr("owner0000".to_string()),
        base_denom: "uusd".to_string(),
    };

    let env = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();
    let msg = ExecuteMsg::Distribute {};

    let env = mock_info("addr0000", &[]);
    let res = execute(deps.as_mut(), env, msg).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("nebula0000"),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: HumanAddr::from("gov0000"),
                amount: Uint128::new(100u128),
                msg: Some(to_binary(&DepositReward {}).unwrap()),
            })
            .unwrap(),
            funds: vec![],
        })]
    )
}
