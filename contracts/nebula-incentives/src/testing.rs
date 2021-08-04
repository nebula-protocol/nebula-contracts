use crate::contract::{handle, init, query_config, query_penalty_period};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::record_contribution;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Decimal, Env, Extern, HumanAddr, Querier, QueryRequest, StdError, Uint128, WasmMsg, WasmQuery, coins, from_binary, log, to_binary};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use terraswap::pair::PoolResponse as TerraswapPoolResponse;


use nebula_protocol::incentives::{ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, PenaltyPeriodResponse, PoolType};
use terraswap::asset::{Asset, AssetInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, HandleMsg as TerraswapHandleMsg};

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InitMsg {
    InitMsg {
        factory: HumanAddr("factory".to_string()),
        custody: HumanAddr("custody".to_string()),
        terraswap_factory: HumanAddr("terraswap_factory".to_string()),
        nebula_token: HumanAddr("nebula_token".to_string()),
        base_denom: "uusd".to_string(),
        owner: HumanAddr("owner0000".to_string()),
    }
}

fn mock_init(mut deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    let msg = init_msg();
    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
}

fn mock_env_height(sender: &str, sent: &[Coin], height: u64, time: u64) -> Env {
    let mut env = mock_env(sender, sent);
    env.block.height = height;
    env.block.time = time;
    env
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();

    let env = mock_env("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(&deps).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0000"),
            factory: HumanAddr("factory".to_string()),
            custody: HumanAddr("custody".to_string()),
            nebula_token: HumanAddr::from("nebula_token"),
            terraswap_factory: HumanAddr::from("terraswap_factory"),
            base_denom: "uusd".to_string(),
        }
    );

    let msg = HandleMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_env("owner0001", &[]);
    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(e) => assert_eq!(e, StdError::unauthorized()),
    }

    let msg = HandleMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(res.log, vec![log("action", "update_owner"),]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(&deps).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            factory: HumanAddr("factory".to_string()),
            custody: HumanAddr("custody".to_string()),
            nebula_token: HumanAddr::from("nebula_token"),
            terraswap_factory: HumanAddr::from("terraswap_factory"),
            base_denom: "uusd".to_string(),
        }
    );
}

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    let rewards_amount = Uint128(1000);
    let total_rewards_amount = Uint128(2000);

    // Send Nebula token to this contract
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_CREATOR),
        amount: total_rewards_amount,
        msg: Some(
            to_binary(&Cw20HookMsg::DepositReward {
                rewards: vec![
                    (
                        PoolType::REBALANCE,
                        HumanAddr::from("cluster"),
                        rewards_amount,
                    ),
                    (
                        PoolType::ARBITRAGE,
                        HumanAddr::from("cluster"),
                        rewards_amount,
                    ),
                ],
            })
            .unwrap(),
        ),
    });
    let env = mock_env(HumanAddr::from("nebula_token"), &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("nebula_token"),
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from("custody"),
                amount: total_rewards_amount,
            })
            .unwrap(),
            send: vec![],
        })]
    );
}

#[test]
fn test_penalty_period() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);
    let msg = HandleMsg::NewPenaltyPeriod {};
    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "new_penalty_period"),
            log("previous_n", 0),
            log("current_n", 1)
        ]
    );

    let penalty_period: PenaltyPeriodResponse = query_penalty_period(&deps).unwrap();
    assert_eq!(penalty_period, PenaltyPeriodResponse { n: 1 });
}

#[test]
fn test_withdraw_reward() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    // First, deposit rewards for both pools
    let rewards_amount = Uint128(1000);
    let total_rewards_amount = Uint128(2000);

    // Send Nebula token to this contract
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_CREATOR),
        amount: total_rewards_amount,
        msg: Some(
            to_binary(&Cw20HookMsg::DepositReward {
                rewards: vec![
                    (
                        PoolType::REBALANCE,
                        HumanAddr::from("cluster"),
                        rewards_amount,
                    ),
                    (
                        PoolType::ARBITRAGE,
                        HumanAddr::from("cluster"),
                        rewards_amount,
                    ),
                ],
            })
            .unwrap(),
        ),
    });
    let env = mock_env(HumanAddr::from("nebula_token"), &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    // Manually record contribution to pools; one pool has other contribution from another address, make sure ratio is correct
    record_contribution(
        &mut deps,
        &HumanAddr::from("contributor0000"),
        PoolType::REBALANCE,
        &HumanAddr::from("cluster"),
        Uint128(25),
    )
    .unwrap();

    record_contribution(
        &mut deps,
        &HumanAddr::from("contributor0000"),
        PoolType::ARBITRAGE,
        &HumanAddr::from("cluster"),
        Uint128(25),
    )
    .unwrap();

    record_contribution(
        &mut deps,
        &HumanAddr::from("contributor0001"),
        PoolType::ARBITRAGE,
        &HumanAddr::from("cluster"),
        Uint128(25),
    )
    .unwrap();

    // Test without advancing penalty period (should give 0)

    let msg = HandleMsg::Withdraw {};
    let env = mock_env("contributor0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(res.log, vec![log("action", "withdraw"), log("amount", 0),]);

    // Advance penalty period

    let msg = HandleMsg::NewPenaltyPeriod {};
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    // Now, we are eligible to collect rewards

    let msg = HandleMsg::Withdraw {};
    let env = mock_env("contributor0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![log("action", "withdraw"), log("amount", 1500),]
    );

    // Make a new penalty period and make sure users can claim past penalties

    let msg = HandleMsg::NewPenaltyPeriod {};
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::Withdraw {};
    let env = mock_env("contributor0001", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![log("action", "withdraw"), log("amount", 500),]
    );
}

/// Integration tests for all mint / redeem operations

#[test]
fn test_incentives_mint() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128(100),
        },
    ];

    let msg = HandleMsg::Mint {
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };
    let env = mock_env("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128(100),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0001"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128(100),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_InternalRewardedMint {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    asset_amounts: asset_amounts,
                    min_tokens: None,
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::_SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: HumanAddr::from("cluster_token"),
                    }],
                    send_to: env.message.sender,
                })
                .unwrap(),
                send: vec![],
            })
        ]
    );
}

#[test]
fn test_incentives_redeem() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("cluster_token"),
        &[(&HumanAddr::from("owner0000"), &Uint128((1000) as u128))],
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128(100),
        },
    ];

    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: HumanAddr::from("asset0000"),
        },
        AssetInfo::Token {
            contract_addr: HumanAddr::from("asset0001"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = HandleMsg::Redeem {
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: Some(asset_amounts.clone()),
        max_tokens: Uint128(1000),
    };

    let env = mock_env("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("cluster_token"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    amount: Uint128(1000),
                    recipient: env.contract.address.clone(),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    max_tokens: Some(Uint128(1000)),
                    asset_amounts: Some(asset_amounts.clone()),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::_SendAll {
                    asset_infos,
                    send_to: env.message.sender,
                })
                .unwrap(),
                send: vec![],
            }),
        ]
    );
}

#[test]
fn test_incentives_arb_cluster_mint() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    deps.querier.with_terraswap_pairs(&[
        (&"uusdcluster_token".to_string(), &HumanAddr::from("uusd_cluster_pair")),
    ]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
            },
            amount: Uint128(100),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "native_asset0000".to_string(),
            },
            amount: Uint128(100),
        },
    ];

    let msg = HandleMsg::ArbClusterMint {
        cluster_contract: HumanAddr::from("cluster"),
        assets: asset_amounts.clone(),
    };
    
    let env = mock_env("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128(100),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0001"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128(100),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_InternalRewardedMint {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    asset_amounts: asset_amounts,
                    min_tokens: None,
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_SwapAll {
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    to_ust: true,
                }).unwrap(),
                send: vec![],
            }),

            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_RecordTerraswapImpact {
                    arbitrager: env.message.sender.clone(),
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_contract: HumanAddr::from("cluster"),
                    pool_before: TerraswapPoolResponse { 
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: HumanAddr::from("cluster_token"),
                                },
                                amount: Uint128(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: Uint128(100),
                            },
                        ], 
                        total_share: Uint128(10000),
                    },
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::_SendAll {
                    asset_infos: vec![AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    }],
                    send_to: env.message.sender,
                })
                .unwrap(),
                send: vec![],
            })
        ]
    );
}

#[test]
fn test_incentives_arb_cluster_redeem() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    deps.querier.with_terraswap_pairs(&[
        (&"uusdcluster_token".to_string(), &HumanAddr::from("uusd_cluster_pair")),
    ]);

    let msg = HandleMsg::ArbClusterRedeem {
        cluster_contract: HumanAddr::from("cluster"),
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128(100),
        },
    };
    
    let env = mock_env("owner0000", &coins(100, &"uusd".to_string()));
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_SwapAll {
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    to_ust: false,
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_RecordTerraswapImpact {
                    arbitrager: env.message.sender.clone(),
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_contract: HumanAddr::from("cluster"),
                    pool_before: TerraswapPoolResponse { 
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: HumanAddr::from("cluster_token"),
                                },
                                amount: Uint128(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: Uint128(100),
                            },
                        ], 
                        total_share: Uint128(10000),
                    },
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&HandleMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    max_tokens: None,
                    asset_amounts: None,
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::_SendAll {
                    asset_infos: vec![
                        AssetInfo::Token {
                                contract_addr: HumanAddr::from("asset0000"),
                        },
                        AssetInfo::Token {
                            contract_addr: HumanAddr::from("asset0001"),
                        },
                        AssetInfo::NativeToken {
                            denom: "native_asset0000".to_string(),
                        },
                    ],
                    send_to: env.message.sender,
                }).unwrap(),
                send: vec![],
            }),
        ]
    );
}


#[test]
fn test_send_all() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("asset0000"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"native_asset0000".to_string(),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"native_asset0000".to_string(), &Uint128(1000000u128))],
    );

    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: HumanAddr::from("asset0000"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = HandleMsg::_SendAll {
        asset_infos: asset_infos.clone(),
        send_to: HumanAddr::from("owner0000"),
    };

    let env = mock_env(MOCK_CONTRACT_ADDR, &vec![]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: HumanAddr::from("owner0000"),
                    amount: Uint128(1000)
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: HumanAddr::from("owner0000"),
                amount: coins(990, &"native_asset0000".to_string()),
            }),
        ]
    );
}

#[test]
fn test_swap_all() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("cluster_token"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"uusd".to_string(),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128(1000000u128))],
    );

    // Test to_ust is true

    let msg = HandleMsg::_SwapAll {
        terraswap_pair: HumanAddr::from("terraswap_pair"),
        cluster_token: HumanAddr::from("cluster_token"),
        to_ust: true,
    };

    let env = mock_env(MOCK_CONTRACT_ADDR, &vec![]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("cluster_token"),
            msg: to_binary(&Cw20HandleMsg::Send {
                contract: HumanAddr::from("terraswap_pair"),
                amount: Uint128(1000),
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
            send: vec![],
        })]
    );

    // Test to_ust is false
    let msg = HandleMsg::_SwapAll {
        terraswap_pair: HumanAddr::from("terraswap_pair"),
        cluster_token: HumanAddr::from("cluster_token"),
        to_ust: false,
    };

    let env = mock_env(MOCK_CONTRACT_ADDR, &vec![]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("terraswap_pair"),
            msg: to_binary(&TerraswapHandleMsg::Swap {
                offer_asset: Asset {
                    amount: Uint128(990),
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    }
                },
                max_spread: None,
                belief_price: None,
                to: None,
            }).unwrap(),
            send: coins(990, &"uusd".to_string()),
        })]
    );
}

// TODO: Specific math tests for cluster_imbalance, terraswap imbalance

// TODO: Specific tests for internal function (SendAll, SwapAll, InternalRewarded*, RecordRewards)

