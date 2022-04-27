use crate::contract::{execute, instantiate};
use crate::error::ContractError;
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::{Asset, AssetInfo};

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, CosmosMsg, Decimal, DepsMut, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::ExecuteMsg as ClusterExecuteMsg;
use nebula_protocol::proxy::{ExecuteMsg, InstantiateMsg};

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".to_string(),
        incentives: None,
        astroport_factory: "astroport_factory".to_string(),
        nebula_token: "nebula_token".to_string(),
        base_denom: "uusd".to_string(),
        owner: "owner0000".to_string(),
    }
}

fn mock_init(deps: DepsMut) {
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps, mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");
}

#[test]
fn test_proxy_arb_cluster_mint_no_incentives() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_astroport_pairs(&[(
        &"uusdcluster_token".to_string(),
        &"uusd_cluster_pair".to_string(),
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128::new(100),
        },
    ];

    let msg = ExecuteMsg::ArbClusterCreate {
        cluster_contract: "cluster".to_string(),
        assets: asset_amounts.clone(),
        min_ust: None,
    };

    let info = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0001".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedCreate {
                    rebalancer: info.sender.clone(),
                    cluster_contract: Addr::unchecked("cluster"),
                    incentives: None,
                    asset_amounts: asset_amounts,
                    min_tokens: None,
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SwapAll {
                    astroport_pair: Addr::unchecked("uusd_cluster_pair"),
                    cluster_token: Addr::unchecked("cluster_token"),
                    to_ust: true,
                    min_return: None,
                    base_denom: "uusd".to_string()
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    }],
                    send_to: info.sender,
                })
                .unwrap(),
                funds: vec![],
            }))
        ]
    );
}

#[test]
fn test_proxy_arb_cluster_redeem_no_incentives() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_astroport_pairs(&[(
        &"uusdcluster_token".to_string(),
        &"uusd_cluster_pair".to_string(),
    )]);

    let msg = ExecuteMsg::ArbClusterRedeem {
        cluster_contract: "cluster".to_string(),
        asset: Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: Uint128::new(100),
        },
        min_cluster: None,
    };

    let info = mock_info("owner0000", &coins(100, &"uusd".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Generic("Not native token".to_string()));

    let msg = ExecuteMsg::ArbClusterRedeem {
        cluster_contract: "cluster".to_string(),
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::new(100),
        },
        min_cluster: None,
    };

    let info = mock_info("owner0000", &coins(100, &"uusd".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SwapAll {
                    astroport_pair: Addr::unchecked("uusd_cluster_pair"),
                    cluster_token: Addr::unchecked("cluster_token"),
                    to_ust: false,
                    min_return: None,
                    base_denom: "uusd".to_string(),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: info.sender.clone(),
                    cluster_contract: Addr::unchecked("cluster"),
                    cluster_token: Addr::unchecked("cluster_token"),
                    incentives: None,
                    max_tokens: None,
                    asset_amounts: None,
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![
                        AssetInfo::Token {
                            contract_addr: Addr::unchecked("asset0000"),
                        },
                        AssetInfo::Token {
                            contract_addr: Addr::unchecked("asset0001"),
                        },
                        AssetInfo::NativeToken {
                            denom: "native_asset0000".to_string(),
                        },
                    ],
                    send_to: info.sender,
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn test_proxy_internal_rewarded_mint_no_incentives() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"native_asset0000".to_string(), &Uint128::new(1000000u128))],
    );

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128::new(100),
        },
    ];

    // perform rebalance create
    let msg = ExecuteMsg::IncentivesCreate {
        cluster_contract: "cluster".to_string(),
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };
    let info = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    let record_msg: ExecuteMsg;
    match res.messages[2].msg.clone() {
        // extract ExecuteMsg::_InternalRewardedCreate
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: _,
            msg,
            funds: _,
        }) => record_msg = from_binary(&msg).unwrap(),
        _ => panic!("DO NOT ENTER HERE"),
    }
    let info = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), record_msg).unwrap();

    let create_asset_amounts_after_tax = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128::new(99),
        },
    ];

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: "cluster".to_string(),
                    amount: Uint128::new(100),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0001".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: "cluster".to_string(),
                    amount: Uint128::new(100),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "cluster".to_string(),
                msg: to_binary(&ClusterExecuteMsg::RebalanceCreate {
                    min_tokens: None,
                    asset_amounts: create_asset_amounts_after_tax,
                })
                .unwrap(),
                funds: coins(99, &"native_asset0000".to_string()),
            })),
        ]
    );
}

#[test]
fn test_proxy_internal_rewarded_redeem_no_incentives() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &"cluster_token".to_string(),
        &[(&"owner0000".to_string(), &Uint128::new((1000) as u128))],
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("asset0001"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128::new(100),
        },
    ];

    // perform rebalance redeem
    let msg = ExecuteMsg::IncentivesRedeem {
        cluster_contract: "cluster".to_string(),
        asset_amounts: Some(asset_amounts.clone()),
        max_tokens: Uint128::new(1000),
    };
    let info = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    let record_msg: ExecuteMsg;
    match res.messages[1].msg.clone() {
        // extract ExecuteMsg::_InternalRewardedRedeem
        CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) => record_msg = from_binary(&msg).unwrap(),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), record_msg).unwrap();

    // Supposedly user cluster tokens are transferred to the contract
    deps.querier.with_token_balances(&[(
        &"cluster_token".to_string(),
        &[
            (&"owner0000".to_string(), &Uint128::new((0) as u128)),
            (
                &MOCK_CONTRACT_ADDR.to_string(),
                &Uint128::new((1000) as u128),
            ),
        ],
    )]);

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "cluster_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: "cluster".to_string(),
                    amount: Uint128::new(1000),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "cluster".to_string(),
                msg: to_binary(&ClusterExecuteMsg::RebalanceRedeem {
                    max_tokens: Uint128::new(1000),
                    asset_amounts: Some(asset_amounts),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: Addr::unchecked("cluster_token"),
                    }],
                    send_to: Addr::unchecked("owner0000"),
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}
