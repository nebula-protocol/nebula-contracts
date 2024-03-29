use crate::contract::{execute, instantiate, migrate, query, query_config};
use crate::error::ContractError;
use crate::state::{contributions_read, read_from_contribution_bucket, record_contribution};
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
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use nebula_protocol::cluster::ExecuteMsg as ClusterExecuteMsg;
use nebula_protocol::incentives::{
    ConfigResponse, ContributorPendingRewardsResponse, CurrentContributorInfoResponse, Cw20HookMsg,
    ExecuteMsg, IncentivesPoolInfoResponse, InstantiateMsg, MigrateMsg, PenaltyPeriodResponse,
    PoolType, QueryMsg,
};
use std::str::FromStr;

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        factory: ("factory".to_string()),
        custody: ("custody".to_string()),
        astroport_factory: ("astroport_factory".to_string()),
        nebula_token: ("nebula_token".to_string()),
        base_denom: "uusd".to_string(),
        owner: ("owner0000".to_string()),
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
            custody: "custody".to_string(),
            nebula_token: "nebula_token".to_string(),
            astroport_factory: "astroport_factory".to_string(),
            base_denom: "uusd".to_string(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: "owner0001".to_string(),
    };

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let msg = ExecuteMsg::UpdateConfig {
        owner: "owner0001".to_string(),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config"),]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            factory: ("factory".to_string()),
            custody: ("custody".to_string()),
            nebula_token: "nebula_token".to_string(),
            astroport_factory: "astroport_factory".to_string(),
            base_denom: "uusd".to_string(),
        }
    );
}

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: total_rewards_amount,
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![
                (PoolType::REBALANCE, "cluster".to_string(), rewards_amount),
                (PoolType::ARBITRAGE, "cluster".to_string(), rewards_amount),
            ],
        })
        .unwrap(),
    });
    let info = mock_info("nebula_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nebula_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "custody".to_string(),
                amount: total_rewards_amount,
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // now we can query the pool info with the new reward amount
    let msg = QueryMsg::PoolInfo {
        pool_type: PoolType::REBALANCE,
        cluster_address: "cluster".to_string(),
        n: None,
    };
    let res: IncentivesPoolInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        IncentivesPoolInfoResponse {
            value_total: Uint128::zero(),
            reward_total: Uint128::new(1000),
        }
    );
}

#[test]
fn test_penalty_period() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());
    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "new_penalty_period"),
            attr("previous_n", "0"),
            attr("current_n", "1")
        ]
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::PenaltyPeriod {}).unwrap();
    let response: PenaltyPeriodResponse = from_binary(&res).unwrap();
    assert_eq!(response, PenaltyPeriodResponse { n: 1 });
}

#[test]
fn test_withdraw_reward() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    // First, deposit rewards for both pools
    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    let deposit_msg = to_binary(&Cw20HookMsg::DepositReward {
        rewards: vec![
            (PoolType::REBALANCE, "cluster".to_string(), rewards_amount),
            (PoolType::ARBITRAGE, "cluster".to_string(), rewards_amount),
        ],
    })
    .unwrap();

    // Specify wrong reward amount
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: Uint128::new(500),
        msg: deposit_msg.clone(),
    });
    let info = mock_info("nebula_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("Rewards amount miss matched".to_string())
    );

    // Send wrong token to this contract
    let info = mock_info("wrong_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: total_rewards_amount,
        msg: deposit_msg,
    });
    let info = mock_info("nebula_token", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Manually record contribution to pools; one pool has other contribution from another address, make sure ratio is correct
    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0000"),
        PoolType::REBALANCE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    // try querying contribution info
    let msg = QueryMsg::CurrentContributorInfo {
        pool_type: PoolType::REBALANCE,
        contributor_address: "contributor0000".to_string(),
        cluster_address: "cluster".to_string(),
    };
    let res: CurrentContributorInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        CurrentContributorInfoResponse {
            n: 0,
            value_contributed: Uint128::new(25)
        }
    );

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0000"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0001"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    // Test without advancing penalty period (should give 0)

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "0"),]
    );

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0000".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::zero());

    // Advance penalty period

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Now, we are eligible to collect rewards

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0000".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::from(1500u128));

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "1500"),]
    );

    // Overwrite the old contribution in the previos penalty periond
    // and move old rewards to `pending_rewards`

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0001"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0001".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::from(500u128));

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "500"),]
    );
}

/// Integration tests for all mint / redeem operations

#[test]
fn test_incentives_mint() {
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
fn test_incentives_redeem() {
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
fn test_incentives_arb_cluster_mint() {
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
                    min_return: None
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
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
fn test_incentives_arb_cluster_redeem() {
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
                    min_return: None
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_RecordAstroportImpact {
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
fn test_incentives_internal_rewarded_mint() {
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
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: Addr::unchecked("rebalancer"),
                    cluster_contract: Addr::unchecked("cluster"),
                    original_imbalance: Uint128::new(51),
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn test_incentives_internal_rewarded_redeem() {
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
        cluster_contract: Addr::unchecked("cluster"),
        asset_amounts: Some(asset_amounts.clone()),
        rebalancer: Addr::unchecked("rebalancer"),
        cluster_token: Addr::unchecked("cluster_token"),
        max_tokens: None,
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
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: Addr::unchecked("rebalancer"),
                    cluster_contract: Addr::unchecked("cluster"),
                    original_imbalance: Uint128::new(51),
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
fn test_record_rebalancer_rewards() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());
    let msg = ExecuteMsg::NewPenaltyPeriod {};

    // unauthorized
    let info = mock_info("imposter0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::_RecordRebalancerRewards {
        cluster_contract: Addr::unchecked("cluster"),
        rebalancer: Addr::unchecked("rebalancer"),
        original_imbalance: Uint128::new(100),
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_rebalancer_rewards"),
            attr("rebalancer_imbalance_fixed", "49"),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        &deps.storage,
        &Addr::unchecked("rebalancer"),
        PoolType::REBALANCE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &Addr::unchecked("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(49));
}

#[test]
fn test_record_astroport_impact() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_astroport_pairs(&[(
        &"uusdcluster_token".to_string(),
        &"uusd_cluster_pair".to_string(),
    )]);

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::_RecordAstroportImpact {
        cluster_contract: Addr::unchecked("cluster"),
        arbitrageur: Addr::unchecked("arbitrageur"),
        astroport_pair: Addr::unchecked("uusd_cluster_pair"),
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
                    amount: Uint128::new(1000),
                },
            ],
            total_share: Uint128::new(100),
        },
    };
    let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_astroport_arbitrageur_rewards"),
            attr("fair_value", "1.6345"),
            attr("arbitrage_imbalance_fixed", "567.862934322973128547"),
            attr("arbitrage_imbalance_sign", "1"),
            attr("imb0", "595.710499796127160136"),
            attr("imb1", "27.847565473154031589"),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        &deps.storage,
        &Addr::unchecked("arbitrageur"),
        PoolType::ARBITRAGE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &Addr::unchecked("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(567));
}

#[test]
fn migration() {
    let mut deps = mock_dependencies(&[]);
    mock_init(deps.as_mut());

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-incentives".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
