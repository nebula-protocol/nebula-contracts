use crate::contract::{execute, instantiate, migrate, query, query_config};
use crate::error::ContractError;
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::{Asset, AssetInfo};
use astroport::pair::{
    Cw20HookMsg as AstroportCw20HookMsg, ExecuteMsg as AstroportExecuteMsg,
    PoolResponse as AstroportPoolResponse,
};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coins, from_binary, to_binary, Addr, BankMsg, CosmosMsg, Decimal, DepsMut, SubMsg,
    Uint128, WasmMsg,
};
use cw2::{get_contract_version, ContractVersion};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::ExecuteMsg as ClusterExecuteMsg;
use nebula_protocol::incentives::ExecuteMsg as IncentivesExecuteMsg;
use nebula_protocol::proxy::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use std::str::FromStr;

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: "factory".to_string(),
        incentives: Some("incentives".to_string()),
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
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = init_msg();

    let info = mock_info("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let msg = QueryMsg::Config {};
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0000".to_string(),
            factory: "factory".to_string(),
            incentives: Some("incentives".to_string()),
            nebula_token: "nebula_token".to_string(),
            astroport_factory: "astroport_factory".to_string(),
            base_denom: "uusd".to_string(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        incentives: Some(Some("new_incentives".to_string())),
    };

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config"),]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            factory: "factory".to_string(),
            incentives: Some("new_incentives".to_string()),
            nebula_token: "nebula_token".to_string(),
            astroport_factory: "astroport_factory".to_string(),
            base_denom: "uusd".to_string(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        incentives: Some(None),
    };

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config")]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            factory: "factory".to_string(),
            incentives: None,
            nebula_token: "nebula_token".to_string(),
            astroport_factory: "astroport_factory".to_string(),
            base_denom: "uusd".to_string(),
        }
    );
}

/// Integration tests for all mint / redeem operations

#[test]
fn test_proxy_mint() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

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

    let msg = ExecuteMsg::IncentivesCreate {
        cluster_contract: "cluster".to_string(),
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
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
                    incentives: Some(Addr::unchecked("incentives")),
                    asset_amounts: asset_amounts,
                    min_tokens: None,
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
                    send_to: info.sender,
                })
                .unwrap(),
                funds: vec![],
            }))
        ]
    );
}

#[test]
fn test_proxy_redeem() {
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

    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0001"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = ExecuteMsg::IncentivesRedeem {
        cluster_contract: "cluster".to_string(),
        asset_amounts: Some(asset_amounts.clone()),
        max_tokens: Uint128::new(1000),
    };

    let info = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "cluster_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    amount: Uint128::new(1000),
                    recipient: env.contract.address.to_string(),
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
                    incentives: Some(Addr::unchecked("incentives")),
                    max_tokens: Some(Uint128::new(1000)),
                    asset_amounts: Some(asset_amounts.clone()),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos,
                    send_to: info.sender,
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn test_proxy_arb_cluster_mint() {
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
                    incentives: Some(Addr::unchecked("incentives")),
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
                contract_addr: "incentives".to_string(),
                msg: to_binary(&IncentivesExecuteMsg::RecordAstroportImpact {
                    arbitrageur: info.sender.clone(),
                    astroport_pair: Addr::unchecked("uusd_cluster_pair"),
                    cluster_contract: Addr::unchecked("cluster"),
                    pool_before: AstroportPoolResponse {
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: Addr::unchecked("cluster_token"),
                                },
                                amount: Uint128::new(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: Uint128::new(100),
                            },
                        ],
                        total_share: Uint128::new(10000),
                    },
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
fn test_proxy_arb_cluster_redeem() {
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
                contract_addr: "incentives".to_string(),
                msg: to_binary(&IncentivesExecuteMsg::RecordAstroportImpact {
                    arbitrageur: info.sender.clone(),
                    astroport_pair: Addr::unchecked("uusd_cluster_pair"),
                    cluster_contract: Addr::unchecked("cluster"),
                    pool_before: AstroportPoolResponse {
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: Addr::unchecked("cluster_token"),
                                },
                                amount: Uint128::new(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: Uint128::new(100),
                            },
                        ],
                        total_share: Uint128::new(10000),
                    },
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
                    incentives: Some(Addr::unchecked("incentives")),
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
fn test_send_all() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &"asset0000".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"native_asset0000".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"native_asset0000".to_string(), &Uint128::new(1000000u128))],
    );

    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: Addr::unchecked("asset0000"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = ExecuteMsg::_SendAll {
        asset_infos: asset_infos.clone(),
        send_to: Addr::unchecked("owner0000"),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "owner0000".to_string(),
                    amount: Uint128::new(1000)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "owner0000".to_string(),
                amount: coins(990, &"native_asset0000".to_string()),
            })),
        ]
    );
}

#[test]
fn test_swap_all() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &"cluster_token".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"uusd".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::new(1000000u128))],
    );

    // Test to_ust is true
    // without min_return
    let msg = ExecuteMsg::_SwapAll {
        astroport_pair: Addr::unchecked("astroport_pair"),
        cluster_token: Addr::unchecked("cluster_token"),
        to_ust: true,
        min_return: None,
        base_denom: "uusd".to_string(),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cluster_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "astroport_pair".to_string(),
                amount: Uint128::new(1000),
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: Some(Decimal::zero()),
                    belief_price: None,
                    to: None,
                })
                .unwrap()
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // with min_return
    let msg = ExecuteMsg::_SwapAll {
        astroport_pair: Addr::unchecked("astroport_pair"),
        cluster_token: Addr::unchecked("cluster_token"),
        to_ust: true,
        min_return: Some(Uint128::new(500)),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cluster_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: "astroport_pair".to_string(),
                amount: Uint128::new(1000),
                msg: to_binary(&AstroportCw20HookMsg::Swap {
                    max_spread: Some(Decimal::zero()),
                    belief_price: Some(Decimal::from_str("2").unwrap()),
                    to: None,
                })
                .unwrap()
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // Test to_ust is false
    // without min_return
    let msg = ExecuteMsg::_SwapAll {
        astroport_pair: Addr::unchecked("astroport_pair"),
        cluster_token: Addr::unchecked("cluster_token"),
        to_ust: false,
        min_return: None,
        base_denom: "uusd".to_string(),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "astroport_pair".to_string(),
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    amount: Uint128::new(990),
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    }
                },
                max_spread: Some(Decimal::zero()),
                belief_price: None,
                to: None,
            })
            .unwrap(),
            funds: coins(990, &"uusd".to_string()),
        }))]
    );

    // with min_return
    let msg = ExecuteMsg::_SwapAll {
        astroport_pair: Addr::unchecked("astroport_pair"),
        cluster_token: Addr::unchecked("cluster_token"),
        to_ust: false,
        min_return: Some(Uint128::new(500)),
        base_denom: "uusd".to_string(),
    };

    let info = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "astroport_pair".to_string(),
            msg: to_binary(&AstroportExecuteMsg::Swap {
                offer_asset: Asset {
                    amount: Uint128::new(990),
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    }
                },
                max_spread: Some(Decimal::zero()),
                belief_price: Some(Decimal::from_str("1.98").unwrap()),
                to: None,
            })
            .unwrap(),
            funds: coins(990, &"uusd".to_string()),
        }))]
    );
}

#[test]
fn test_proxy_internal_rewarded_mint() {
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

    let msg = ExecuteMsg::_InternalRewardedCreate {
        cluster_contract: Addr::unchecked("cluster"),
        asset_amounts: asset_amounts.clone(),
        incentives: Some(Addr::unchecked("incentives")),
        min_tokens: None,
        rebalancer: Addr::unchecked("rebalancer"),
    };
    let info = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

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
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "incentives".to_string(),
                msg: to_binary(&IncentivesExecuteMsg::RecordRebalancerRewards {
                    rebalancer: Addr::unchecked("rebalancer"),
                    cluster_contract: Addr::unchecked("cluster"),
                    original_inventory: vec![
                        Uint128::new(110),
                        Uint128::new(100),
                        Uint128::new(95)
                    ],
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn test_proxy_internal_rewarded_redeem() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &"cluster_token".to_string(),
        &[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &Uint128::new((1000) as u128),
        )],
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

    let msg = ExecuteMsg::_InternalRewardedRedeem {
        rebalancer: Addr::unchecked("rebalancer"),
        cluster_contract: Addr::unchecked("cluster"),
        cluster_token: Addr::unchecked("cluster_token"),
        incentives: Some(Addr::unchecked("incentives")),
        max_tokens: None,
        asset_amounts: Some(asset_amounts.clone()),
    };
    let info = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();

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
                contract_addr: "incentives".to_string(),
                msg: to_binary(&IncentivesExecuteMsg::RecordRebalancerRewards {
                    rebalancer: Addr::unchecked("rebalancer"),
                    cluster_contract: Addr::unchecked("cluster"),
                    original_inventory: vec![
                        Uint128::new(110),
                        Uint128::new(100),
                        Uint128::new(95)
                    ],
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
                    send_to: Addr::unchecked("rebalancer"),
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn migration() {
    let mut deps = mock_dependencies(&[]);
    mock_init(deps.as_mut());

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-proxy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
