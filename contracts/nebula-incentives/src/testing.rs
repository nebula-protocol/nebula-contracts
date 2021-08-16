use crate::contract::{execute, init, query, query_config, query_penalty_period};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{contributions_read, read_from_contribution_bucket, record_contribution};
use cosmwasm_std::testing::{mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, to_binary, BankMsg, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    HumanAddr, QueryRequest, StdError, Uint128, WasmMsg, WasmQuery,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use nebula_protocol::cluster::{
    ClusterStateResponse, ExecuteMsg as ClusterExecuteMsg, QueryMsg as ClusterQueryMsg,
};
use terraswap::pair::PoolResponse as TerraswapPoolResponse;

use nebula_protocol::incentives::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, PenaltyPeriodResponse, PoolType,
    QueryMsg,
};
use terraswap::asset::{Asset, AssetInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, ExecuteMsg as TerraswapExecuteMsg};

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
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
    let env = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps.as_mut(), env, msg)
        .expect("contract successfully executes InstantiateMsg");
}

fn mock_info_height(sender: &str, sent: &[Coin], height: u64, time: u64) -> Env {
    let mut env = mock_info(sender, sent);
    env.block.height = height;
    (env.block.time.nanos() / 1_000_000_000) = time;
    env
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();

    let env = mock_info("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
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

    let msg = ExecuteMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(e) => assert_eq!(e, StdError::unauthorized()),
    }

    let msg = ExecuteMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), env, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_owner"),]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
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

    mock_init(deps.as_mut());

    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
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
    let env = mock_info(HumanAddr::from("nebula_token"), &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("nebula_token"),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: HumanAddr::from("custody"),
                amount: total_rewards_amount,
            })
            .unwrap(),
            funds: vec![],
        })]
    );
}

#[test]
fn test_penalty_period() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());
    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let env = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "new_penalty_period"),
            attr("previous_n", 0),
            attr("current_n", 1)
        ]
    );

    let res = query(deps.as_ref(), QueryMsg::PenaltyPeriod {}).unwrap();
    let response: PenaltyPeriodResponse = from_binary(&res).unwrap();
    assert_eq!(response, PenaltyPeriodResponse { n: 1 });
}

#[test]
fn test_withdraw_reward() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    // First, deposit rewards for both pools
    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
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
    let env = mock_info(HumanAddr::from("nebula_token"), &[]);
    let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    // Manually record contribution to pools; one pool has other contribution from another address, make sure ratio is correct
    record_contribution(
        deps.as_mut(),
        &HumanAddr::from("contributor0000"),
        PoolType::REBALANCE,
        &HumanAddr::from("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    record_contribution(
        deps.as_mut(),
        &HumanAddr::from("contributor0000"),
        PoolType::ARBITRAGE,
        &HumanAddr::from("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    record_contribution(
        deps.as_mut(),
        &HumanAddr::from("contributor0001"),
        PoolType::ARBITRAGE,
        &HumanAddr::from("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    // Test without advancing penalty period (should give 0)

    let msg = ExecuteMsg::Withdraw {};
    let env = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", 0),]
    );

    // Advance penalty period

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let env = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    // Now, we are eligible to collect rewards

    let msg = ExecuteMsg::Withdraw {};
    let env = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", 1500),]
    );

    // Make a new penalty period and make sure users can claim past penalties

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let env = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::Withdraw {};
    let env = mock_info("contributor0001", &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", 500),]
    );
}

/// Integration tests for all mint / redeem operations

#[test]
fn test_incentives_mint() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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

    let msg = ExecuteMsg::Mint {
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };
    let env = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0001"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedMint {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    asset_amounts: asset_amounts,
                    min_tokens: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: HumanAddr::from("cluster_token"),
                    }],
                    send_to: env.message.sender,
                })
                .unwrap(),
                funds: vec![],
            })
        ]
    );
}

#[test]
fn test_incentives_redeem() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("cluster_token"),
        &[(&HumanAddr::from("owner0000"), &Uint128::new((1000) as u128))],
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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
            contract_addr: HumanAddr::from("asset0000"),
        },
        AssetInfo::Token {
            contract_addr: HumanAddr::from("asset0001"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = ExecuteMsg::Redeem {
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: Some(asset_amounts.clone()),
        max_tokens: Uint128::new(1000),
    };

    let env = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 3);

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("cluster_token"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    amount: Uint128::new(1000),
                    recipient: env.contract.address.clone(),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    max_tokens: Some(Uint128::new(1000)),
                    asset_amounts: Some(asset_amounts.clone()),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos,
                    send_to: env.message.sender,
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );
}

#[test]
fn test_incentives_arb_cluster_mint() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_terraswap_pairs(&[(
        &"uusdcluster_token".to_string(),
        &HumanAddr::from("uusd_cluster_pair"),
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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

    let msg = ExecuteMsg::ArbClusterMint {
        cluster_contract: HumanAddr::from("cluster"),
        assets: asset_amounts.clone(),
        min_ust: None,
    };

    let env = mock_info("owner0000", &coins(100, &"native_asset0000".to_string()));
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0001"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: env.message.sender.clone(),
                    recipient: env.contract.address.clone(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedMint {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    asset_amounts: asset_amounts,
                    min_tokens: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_SwapAll {
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    to_ust: true,
                    min_return: Uint128::zero()
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_RecordTerraswapImpact {
                    arbitrageur: env.message.sender.clone(),
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_contract: HumanAddr::from("cluster"),
                    pool_before: TerraswapPoolResponse {
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: HumanAddr::from("cluster_token"),
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
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    }],
                    send_to: env.message.sender,
                })
                .unwrap(),
                funds: vec![],
            })
        ]
    );
}

#[test]
fn test_incentives_arb_cluster_redeem() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_terraswap_pairs(&[(
        &"uusdcluster_token".to_string(),
        &HumanAddr::from("uusd_cluster_pair"),
    )]);

    let msg = ExecuteMsg::ArbClusterRedeem {
        cluster_contract: HumanAddr::from("cluster"),
        asset: Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        min_cluster: None,
    };

    let env = mock_info("owner0000", &coins(100, &"uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), msg.clone());

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "not native token"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }

    let msg = ExecuteMsg::ArbClusterRedeem {
        cluster_contract: HumanAddr::from("cluster"),
        asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            amount: Uint128::new(100),
        },
        min_cluster: None,
    };

    let env = mock_info("owner0000", &coins(100, &"uusd".to_string()));
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_SwapAll {
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    to_ust: false,
                    min_return: Uint128::zero()
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_RecordTerraswapImpact {
                    arbitrageur: env.message.sender.clone(),
                    terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
                    cluster_contract: HumanAddr::from("cluster"),
                    pool_before: TerraswapPoolResponse {
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: HumanAddr::from("cluster_token"),
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
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_InternalRewardedRedeem {
                    rebalancer: env.message.sender.clone(),
                    cluster_contract: HumanAddr::from("cluster"),
                    cluster_token: HumanAddr::from("cluster_token"),
                    max_tokens: None,
                    asset_amounts: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
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
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );
}

#[test]
fn test_send_all() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("asset0000"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"native_asset0000".to_string(),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"native_asset0000".to_string(), &Uint128::new(1000000u128))],
    );

    let asset_infos = vec![
        AssetInfo::Token {
            contract_addr: HumanAddr::from("asset0000"),
        },
        AssetInfo::NativeToken {
            denom: "native_asset0000".to_string(),
        },
    ];

    let msg = ExecuteMsg::_SendAll {
        asset_infos: asset_infos.clone(),
        send_to: HumanAddr::from("owner0000"),
    };

    let env = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(res.messages.len(), 2);

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: HumanAddr::from("owner0000"),
                    amount: Uint128::new(1000)
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: HumanAddr::from("owner0000"),
                amount: coins(990, &"native_asset0000".to_string()),
            }),
        ]
    );
}

#[test]
fn test_swap_all() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("cluster_token"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_native_balances(&[(
        &"uusd".to_string(),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::new((1000) as u128),
        )],
    )]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::new(1000000u128))],
    );

    // Test to_ust is true

    let msg = ExecuteMsg::_SwapAll {
        terraswap_pair: HumanAddr::from("terraswap_pair"),
        cluster_token: HumanAddr::from("cluster_token"),
        to_ust: true,
        min_return: Uint128::zero(),
    };

    let env = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("cluster_token"),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: HumanAddr::from("terraswap_pair"),
                amount: Uint128::new(1000),
                msg: Some(
                    to_binary(&TerraswapCw20HookMsg::Swap {
                        max_spread: Some(Decimal::zero()),
                        belief_price: Some(Decimal::zero()),
                        to: None,
                    })
                    .unwrap()
                ),
            })
            .unwrap(),
            funds: vec![],
        })]
    );

    // Test to_ust is false
    let msg = ExecuteMsg::_SwapAll {
        terraswap_pair: HumanAddr::from("terraswap_pair"),
        cluster_token: HumanAddr::from("cluster_token"),
        to_ust: false,
        min_return: Uint128::zero(),
    };

    let env = mock_info(MOCK_CONTRACT_ADDR, &vec![]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("terraswap_pair"),
            msg: to_binary(&TerraswapExecuteMsg::Swap {
                offer_asset: Asset {
                    amount: Uint128::new(990),
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string()
                    }
                },
                max_spread: Some(Decimal::zero()),
                belief_price: Some(Decimal::zero()),
                to: None,
            })
            .unwrap(),
            send: coins(990, &"uusd".to_string()),
        })]
    );
}

#[test]
fn test_incentives_internal_rewarded_mint() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"native_asset0000".to_string(), &Uint128::new(1000000u128))],
    );

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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

    let msg = ExecuteMsg::_InternalRewardedMint {
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
        rebalancer: HumanAddr::from("rebalancer"),
    };
    let env = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    let mint_asset_amounts_after_tax = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0000"),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: HumanAddr::from("cluster"),
                    amount: Uint128::new(100),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("asset0001"),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: HumanAddr::from("cluster"),
                    amount: Uint128::new(100),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("cluster"),
                msg: to_binary(&ClusterExecuteMsg::Mint {
                    min_tokens: None,
                    asset_amounts: mint_asset_amounts_after_tax,
                })
                .unwrap(),
                send: coins(99, &"native_asset0000".to_string()),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: HumanAddr::from("rebalancer"),
                    cluster_contract: HumanAddr::from("cluster"),
                    original_imbalance: Uint128::new(51),
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );
}

#[test]
fn test_incentives_internal_rewarded_redeem() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_token_balances(&[(
        &HumanAddr::from("cluster_token"),
        &[(
            &HumanAddr::from(MOCK_CONTRACT_ADDR),
            &Uint128::new((1000) as u128),
        )],
    )]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0000"),
            },
            amount: Uint128::new(100),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: HumanAddr::from("asset0001"),
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
        cluster_contract: HumanAddr::from("cluster"),
        asset_amounts: Some(asset_amounts.clone()),
        rebalancer: HumanAddr::from("rebalancer"),
        cluster_token: HumanAddr::from("cluster_token"),
        max_tokens: None,
    };
    let env = mock_info(
        MOCK_CONTRACT_ADDR,
        &coins(100, &"native_asset0000".to_string()),
    );
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("cluster_token"),
                msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                    spender: HumanAddr::from("cluster"),
                    amount: Uint128::new(1000),
                    expires: None,
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("cluster"),
                msg: to_binary(&ClusterExecuteMsg::Burn {
                    max_tokens: Uint128::new(1000),
                    asset_amounts: Some(asset_amounts),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.clone(),
                msg: to_binary(&ExecuteMsg::_RecordRebalancerRewards {
                    rebalancer: HumanAddr::from("rebalancer"),
                    cluster_contract: HumanAddr::from("cluster"),
                    original_imbalance: Uint128::new(51),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address,
                msg: to_binary(&ExecuteMsg::_SendAll {
                    asset_infos: vec![AssetInfo::Token {
                        contract_addr: HumanAddr::from("cluster_token"),
                    }],
                    send_to: HumanAddr::from("rebalancer"),
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );
}

#[test]
fn test_record_rebalancer_rewards() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let env = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::_RecordRebalancerRewards {
        cluster_contract: HumanAddr::from("cluster"),
        rebalancer: HumanAddr::from("rebalancer"),
        original_imbalance: Uint128::new(100),
    };
    let env = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_rebalancer_rewards"),
            attr("rebalancer_imbalance_fixed", 49),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        deps.storage,
        &HumanAddr::from("rebalancer"),
        PoolType::REBALANCE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &HumanAddr::from("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(49));
}

#[test]
fn test_record_terraswap_impact() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(deps.as_mut());

    deps.querier.with_terraswap_pairs(&[(
        &"uusdcluster_token".to_string(),
        &HumanAddr::from("uusd_cluster_pair"),
    )]);

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let env = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::_RecordTerraswapImpact {
        cluster_contract: HumanAddr::from("cluster"),
        arbitrageur: HumanAddr::from("arbitrageur"),
        terraswap_pair: HumanAddr::from("uusd_cluster_pair"),
        pool_before: TerraswapPoolResponse {
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: HumanAddr::from("cluster_token"),
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
    let env = mock_info(MOCK_CONTRACT_ADDR, &[]);
    let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_terraswap_arbitrageur_rewards"),
            attr("fair_value", "1.6345"),
            attr("arbitrage_imbalance_fixed", "567.862934322973128547"),
            attr("arbitrage_imbalance_sign", "1"),
            attr("imb0", "595.710499796127160136"),
            attr("imb1", "27.847565473154031589"),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        deps.storage,
        &HumanAddr::from("arbitrageur"),
        PoolType::ARBITRAGE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &HumanAddr::from("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(567));
}
