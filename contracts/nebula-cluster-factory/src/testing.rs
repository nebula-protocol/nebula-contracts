use crate::contract::{handle, init, query};
use crate::mock_querier::mock_dependencies;

use crate::state::{cluster_exists, read_params, read_total_weight, read_weight, store_total_weight, store_weight};
use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::Api;
use cosmwasm_std::{
    from_binary, log, to_binary, CosmosMsg, Decimal, Env, HumanAddr, StdError, Uint128, WasmMsg,
};
use cw20::{Cw20HandleMsg, MinterResponse};

use nebula_protocol::cluster_factory::{
    ConfigResponse, DistributionInfoResponse, HandleMsg, InitMsg, Params, QueryMsg,
};

use nebula_protocol::cluster::{HandleMsg as ClusterHandleMsg, InitMsg as ClusterInitMsg};
use nebula_protocol::oracle::HandleMsg as OracleHandleMsg;
use nebula_protocol::staking::{Cw20HookMsg as StakingCw20HookMsg, HandleMsg as StakingHandleMsg};
use nebula_protocol::penalty::HandleMsg as PenaltyHandleMsg;
use terraswap::asset::{Asset, AssetInfo};
use terraswap::factory::HandleMsg as TerraswapFactoryHandleMsg;
use terraswap::hook::InitHook;
use terraswap::token::InitMsg as TokenInitMsg;

fn mock_env_time(signer: &HumanAddr, time: u64) -> Env {
    let mut env = mock_env(signer, &[]);
    env.block.time = time;
    env
}

/// Convenience function for creating inline HumanAddr
pub fn h(s: &str) -> HumanAddr {
    HumanAddr(s.to_string())
}

pub fn get_input_params() -> Params {
    Params {
        name: "Test Cluster".to_string(),
        symbol: "TEST".to_string(),
        description: "Sample cluster for testing".to_string(),
        weight: Some(100u32),
        penalty: HumanAddr::from("penalty0000"),
        pricing_oracle: HumanAddr::from("pricing_oracle0000"),
        composition_oracle: HumanAddr::from("comp_oracle0000"),
        target: vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128(20),
            },
        ]
    }
}

static TOKEN_CODE_ID: u64 = 8u64;
static CLUSTER_CODE_ID: u64 = 1u64;
static BASE_DENOM: &str = "uusd";
static PROTOCOL_FEE_RATE: &str = "0.01";

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // cannot update mirror token after initialization
    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
    };
    let _res = handle(&mut deps, env, msg).unwrap_err();

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0000"),
            nebula_token: HumanAddr::from("nebula0000"),
            staking_contract: HumanAddr::from("staking0000"),
            commission_collector: HumanAddr::from("collector0000"),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            terraswap_factory: HumanAddr::from("terraswapfactory"),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID,
            cluster_code_id: CLUSTER_CODE_ID,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![],
        }
    );
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // upate owner
    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr::from("owner0001")),
        distribution_schedule: None,
        token_code_id: None,
        cluster_code_id: None,
    };

    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            nebula_token: HumanAddr::from("nebula0000"),
            staking_contract: HumanAddr::from("staking0000"),
            commission_collector: HumanAddr::from("collector0000"),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            terraswap_factory: HumanAddr::from("terraswapfactory"),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID,
            cluster_code_id: CLUSTER_CODE_ID,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![],
        }
    );

    // update rest part
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        distribution_schedule: Some(vec![(1, 2, Uint128::from(123u128))]),
        token_code_id: Some(TOKEN_CODE_ID + 1),
        cluster_code_id: Some(CLUSTER_CODE_ID + 1),
    };

    let env = mock_env("owner0001", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            nebula_token: HumanAddr::from("nebula0000"),
            staking_contract: HumanAddr::from("staking0000"),
            commission_collector: HumanAddr::from("collector0000"),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            terraswap_factory: HumanAddr::from("terraswapfactory"),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID + 1,
            cluster_code_id: CLUSTER_CODE_ID + 1,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![(1, 2, Uint128::from(123u128))],
        }
    );

    // failed unauthoirzed
    let msg = HandleMsg::UpdateConfig {
        owner: None,
        distribution_schedule: None,
        token_code_id: Some(TOKEN_CODE_ID + 1),
        cluster_code_id: Some(CLUSTER_CODE_ID + 1),
    };

    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap_err();
    match res {
        StdError::Unauthorized { .. } => {}
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_update_weight() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };
    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    store_total_weight(&mut deps.storage, 100).unwrap();
    store_weight(&mut deps.storage, &HumanAddr::from("asset0000"), 10).unwrap();

    // incrase weight
    let msg = HandleMsg::UpdateWeight {
        asset_token: HumanAddr::from("asset0000"),
        weight: 20,
    };
    let env = mock_env("owner0001", &[]);
    let res = handle(&mut deps, env, msg.clone()).unwrap_err();
    match res {
        StdError::Unauthorized { .. } => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();
    let res = query(&deps, QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(HumanAddr::from("asset0000"), 20)],
            last_distributed: 1_571_797_419,
        }
    );

    assert_eq!(
        read_weight(&deps.storage, &HumanAddr::from("asset0000")).unwrap(),
        20u32
    );
    assert_eq!(read_total_weight(&deps.storage).unwrap(), 110u32);
}

#[test]
fn test_create_cluster() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };

    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let input_params: Params = get_input_params();
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "create_cluster"),
            log("symbol", "TEST"),
            log("name", "Test Cluster")
        ]
    );

    // token creation msg should be returned
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: CLUSTER_CODE_ID,
            send: vec![],
            label: None,
            msg: to_binary(&ClusterInitMsg {
                name: input_params.name.clone(),
                description: input_params.description.clone(),
                owner: h(MOCK_CONTRACT_ADDR),
                pricing_oracle: input_params.pricing_oracle.clone(),
                composition_oracle: input_params.composition_oracle.clone(),
                penalty: input_params.penalty.clone(),
                factory: h(MOCK_CONTRACT_ADDR),
                cluster_token: None,
                target: input_params.target.clone(),
                init_hook: Some(InitHook {
                    contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&HandleMsg::TokenCreationHook {})
                    .unwrap(),
                }),
            })
            .unwrap(),
        })]
    );

    let params: Params = read_params(&deps.storage).unwrap();
    assert_eq!(
        params,
        input_params
    );

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "A cluster registration process is in progress"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env("addr0001", &[]);
    let res = handle(&mut deps, env, msg).unwrap_err();
    match res {
        StdError::Unauthorized { .. } => {}
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_token_creation_hook() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };

    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // There is no cluster registration process; failed
    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "No cluster registration process in progress")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let input_params: Params = get_input_params();
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    
    let env = mock_env("asset0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: input_params.penalty.clone(),
                send: vec![],
                msg: to_binary(&PenaltyHandleMsg::UpdateConfig {
                    owner: Some(h("asset0000")),
                    penalty_params: None,
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: TOKEN_CODE_ID,
                send: vec![],
                label: None,
                msg: to_binary(&TokenInitMsg {
                    name: input_params.name.clone(),
                    symbol: input_params.symbol.clone(),
                    decimals: 6u8,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: h("asset0000"),
                        cap: None,
                    }),
                    // Set Cluster Token
                    init_hook: Some(InitHook {
                        contract_addr: h(MOCK_CONTRACT_ADDR),
                        msg: to_binary(&HandleMsg::SetClusterTokenHook {
                            cluster: h("asset0000"),
                        })
                        .unwrap(),
                    }),
                })
                .unwrap(),
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("asset0000"),
                send: vec![],
                msg: to_binary(&ClusterHandleMsg::UpdateConfig {
                    owner: Some(h("owner0000")),
                    name: None,
                    description: None,
                    cluster_token: None,
                    pricing_oracle: None,
                    composition_oracle: None,
                    penalty: None,
                    target: None,
                })
                .unwrap(),
            }),
        ]
    );
    
    assert_eq!(
        res.log,
        vec![log("cluster_addr", "asset0000")]
    );

    assert_eq!(
        cluster_exists(&deps.storage, &h("asset0000")),
        Ok(true)
    )
}

#[test]
fn test_set_cluster_token_hook() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };

    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // There is no cluster registration process; failed
    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "No cluster registration process in progress")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let input_params: Params = get_input_params();
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h("asset0000"),
            send: vec![],
            msg: to_binary(&ClusterHandleMsg::UpdateConfig {
                owner: None,
                name: None,
                description: None,
                cluster_token: Some(h("cluster_token0000")),
                pricing_oracle: None,
                composition_oracle: None,
                penalty: None,
                target: None,
            })
            .unwrap(),
        }),
        // set up terraswap pair
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h("terraswapfactory"),
            send: vec![],
            msg: to_binary(&TerraswapFactoryHandleMsg::CreatePair {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: BASE_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: h("cluster_token0000"),
                    },
                ],
                init_hook: Some(InitHook {
                    msg: to_binary(&HandleMsg::TerraswapCreationHook {
                        asset_token: h("cluster_token0000"),
                    })
                    .unwrap(),
                    contract_addr: h(MOCK_CONTRACT_ADDR),
                }),
            })
            .unwrap(),
        })]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "set_cluster_token"),
            log("cluster", "asset0000"),
            log("token", "cluster_token0000")
        ]
    );

    let res = query(&deps, QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(HumanAddr::from("cluster_token0000"), 100)],
            last_distributed: 1_571_797_419,
        }
    );

    // After execution of these hook, params is removed so we check that 
    // there is no cluster registration process; failed
    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "No cluster registration process in progress")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_set_cluster_token_hook_without_weight() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };

    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    // There is no cluster registration process; failed
    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "No cluster registration process in progress")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let mut input_params: Params = get_input_params();
    input_params.weight = None;
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h("asset0000"),
            send: vec![],
            msg: to_binary(&ClusterHandleMsg::UpdateConfig {
                owner: None,
                name: None,
                description: None,
                cluster_token: Some(h("cluster_token0000")),
                pricing_oracle: None,
                composition_oracle: None,
                penalty: None,
                target: None,
            })
            .unwrap(),
        }),
        // set up terraswap pair
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h("terraswapfactory"),
            send: vec![],
            msg: to_binary(&TerraswapFactoryHandleMsg::CreatePair {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: BASE_DENOM.to_string(),
                    },
                    AssetInfo::Token {
                        contract_addr: h("cluster_token0000"),
                    },
                ],
                init_hook: Some(InitHook {
                    msg: to_binary(&HandleMsg::TerraswapCreationHook {
                        asset_token: h("cluster_token0000"),
                    })
                    .unwrap(),
                    contract_addr: h(MOCK_CONTRACT_ADDR),
                }),
            })
            .unwrap(),
        })]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "set_cluster_token"),
            log("cluster", "asset0000"),
            log("token", "cluster_token0000")
        ]
    );

    let res = query(&deps, QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(HumanAddr::from("cluster_token0000"), 30)],
            last_distributed: 1_571_797_419,
        }
    );

    // After execution of these hook, params is removed so we check that 
    // there is no cluster registration process; failed
    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "No cluster registration process in progress")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }
}

#[test]
fn test_terraswap_creation_hook() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier
        .with_terraswap_pairs(&[(&"uusdasset0000".to_string(), &HumanAddr::from("LP0000"))]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };

    let _res = handle(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::TerraswapCreationHook {
        asset_token: HumanAddr::from("asset0000"),
    };
    
    let env = mock_env("terraswapfactory1", &[]);
    let res = handle(&mut deps, env, msg).unwrap_err();

    match res {
        StdError::Unauthorized { .. } => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let input_params: Params = get_input_params();
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    
    let env = mock_env("cluster_token0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::TerraswapCreationHook {
        asset_token: HumanAddr::from("asset0000"),
    };
    
    let env = mock_env("terraswapfactory", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("staking0000"),
            send: vec![],
            msg: to_binary(&StakingHandleMsg::RegisterAsset {
                asset_token: HumanAddr::from("asset0000"),
                staking_token: HumanAddr::from("LP0000"),
            })
            .unwrap(),
        })]
    );
}

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(20, &[]);
    deps.querier.with_terraswap_pairs(&[
        (&"uusdasset0000".to_string(), &HumanAddr::from("LP0000")),
        (&"uusdasset0001".to_string(), &HumanAddr::from("LP0001")),
    ]);

    let msg = InitMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![
            (1800, 3600, Uint128::from(3600u128)),
            (3600, 3600 + 3600, Uint128::from(7200u128)),
        ]
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    let msg = HandleMsg::PostInitialize {
        owner: HumanAddr::from("owner0000"),
        nebula_token: HumanAddr::from("nebula0000"),
        staking_contract: HumanAddr::from("staking0000"),
        commission_collector: HumanAddr::from("collector0000"),
        terraswap_factory: HumanAddr::from("terraswapfactory"),
    };
    let _res = handle(&mut deps, env, msg).unwrap();

    // create first cluter with weight 30
    let input_params: Params = get_input_params();
    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0000"),
    };
    
    let env = mock_env("cluster_token0000", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::TerraswapCreationHook {
        asset_token: HumanAddr::from("asset0000"),
    };
    
    let env = mock_env("terraswapfactory", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // create second cluter with weight 30
    let mut input_params: Params = get_input_params();
    input_params.weight = Some(30u32);
    input_params.name = "Test Cluster 2".to_string();
    input_params.symbol = "TEST2".to_string();

    let msg = HandleMsg::CreateCluster {
        params: input_params.clone()
    };
    let env = mock_env("owner0000", &[]);
    let _res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    let msg = HandleMsg::TokenCreationHook {};
    let env = mock_env("asset0001", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::SetClusterTokenHook {
        cluster: h("asset0001"),
    };
    
    let env = mock_env("cluster_token0001", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::TerraswapCreationHook {
        asset_token: HumanAddr::from("asset0001"),
    };
    
    let env = mock_env("terraswapfactory", &[]);
    let _res = handle(&mut deps, env, msg).unwrap();

    // height is not increased so zero amount will be minted
    let msg = HandleMsg::Distribute {};
    let env = mock_env("anyone", &[]);
    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Cannot distribute nebula token before interval")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    // one height increase
    let msg = HandleMsg::Distribute {};
    let env = mock_env_time(&HumanAddr::from("addr0000"), 1_571_797_419u64 + 5400u64);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "distribute"),
            log("distribution_amount", "7199"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("nebula0000"),
                msg: to_binary(&Cw20HandleMsg::Send {
                    contract: h("staking0000"),
                    amount: Uint128(7199u128),
                    msg: Some(to_binary(&StakingCw20HookMsg::DepositReward { 
                        rewards: vec![
                        (HumanAddr::from("cluster_token0000"), Uint128(5538)),
                        (HumanAddr::from("cluster_token0001"), Uint128(1661)),
                        ], 
                    })
                    .unwrap()
                ),
            })
            .unwrap(),
            send: vec![],
        }),],
    );

    let res = query(&deps, QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![
                (HumanAddr::from("cluster_token0000"), 100),
                (HumanAddr::from("cluster_token0001"), 30)
            ],
            last_distributed: 1_571_802_819,
        }
    );
}

// #[test]
// fn test_revocation() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier
//         .with_terraswap_pairs(&[(&"uusdasset0000".to_string(), &HumanAddr::from("LP0000"))]);

//     let msg = InitMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         distribution_schedule: vec![],
//     };

//     let env = mock_env("addr0000", &[]);
//     let _res = init(&mut deps, env.clone(), msg).unwrap();

//     let msg = HandleMsg::PostInitialize {
//         owner: HumanAddr::from("owner0000"),
//         nebula_token: HumanAddr::from("nebula0000"),
//         mint_contract: HumanAddr::from("mint0000"),
//         staking_contract: HumanAddr::from("staking0000"),
//         commission_collector: HumanAddr::from("collector0000"),
//         oracle_contract: HumanAddr::from("oracle0000"),
//         terraswap_factory: HumanAddr::from("terraswapfactory"),
//     };
//     let _res = handle(&mut deps, env, msg).unwrap();

//     // whitelist first item with weight 1.5
//     let msg = HandleMsg::Whitelist {
//         name: "apple derivative".to_string(),
//         symbol: "mAAPL".to_string(),
//         oracle_feeder: HumanAddr::from("feeder0000"),
//         params: Params {
//             auction_discount: Decimal::percent(5),
//             min_collateral_ratio: Decimal::percent(150),
//             weight: Some(100u32),
//             mint_period: None,
//             min_collateral_ratio_after_migration: None,
//         },
//     };
//     let env = mock_env("owner0000", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::TokenCreationHook {
//         oracle_feeder: HumanAddr::from("feeder0000"),
//     };
//     let env = mock_env("asset0000", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::TerraswapCreationHook {
//         asset_token: HumanAddr::from("asset0000"),
//     };
//     let env = mock_env("terraswapfactory", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();
//     // register queriers
//     deps.querier.with_oracle_feeders(&[(
//         &HumanAddr::from("asset0000"),
//         &HumanAddr::from("feeder0000"),
//     )]);

//     // unauthorized revoke attempt
//     let msg = HandleMsg::RevokeAsset {
//         asset_token: HumanAddr::from("asset0000"),
//         end_price: Decimal::from_ratio(2u128, 1u128),
//     };
//     let env = mock_env("owner0000", &[]);
//     let res = handle(&mut deps, env, msg.clone()).unwrap_err();

//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let env = mock_env("feeder0000", &[]);
//     let res = handle(&mut deps, env, msg).unwrap();
//     assert_eq!(
//         res.messages,
//         vec![CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: HumanAddr::from("mint0000"),
//             send: vec![],
//             msg: to_binary(&MintHandleMsg::RegisterMigration {
//                 asset_token: HumanAddr::from("asset0000"),
//                 end_price: Decimal::from_ratio(2u128, 1u128),
//             })
//             .unwrap(),
//         }),]
//     );
// }

// #[test]
// fn test_migration() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier
//         .with_terraswap_pairs(&[(&"uusdasset0000".to_string(), &HumanAddr::from("LP0000"))]);

//     let msg = InitMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         distribution_schedule: vec![],
//     };

//     let env = mock_env("addr0000", &[]);
//     let _res = init(&mut deps, env.clone(), msg).unwrap();

//     let msg = HandleMsg::PostInitialize {
//         owner: HumanAddr::from("owner0000"),
//         nebula_token: HumanAddr::from("nebula0000"),
//         mint_contract: HumanAddr::from("mint0000"),
//         staking_contract: HumanAddr::from("staking0000"),
//         commission_collector: HumanAddr::from("collector0000"),
//         oracle_contract: HumanAddr::from("oracle0000"),
//         terraswap_factory: HumanAddr::from("terraswapfactory"),
//     };
//     let _res = handle(&mut deps, env, msg).unwrap();

//     // whitelist first item with weight 1.5
//     let msg = HandleMsg::Whitelist {
//         name: "apple derivative".to_string(),
//         symbol: "mAAPL".to_string(),
//         oracle_feeder: HumanAddr::from("feeder0000"),
//         params: Params {
//             auction_discount: Decimal::percent(5),
//             min_collateral_ratio: Decimal::percent(150),
//             weight: Some(100u32),
//             mint_period: None,
//             min_collateral_ratio_after_migration: None,
//         },
//     };
//     let env = mock_env("owner0000", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::TokenCreationHook {
//         oracle_feeder: HumanAddr::from("feeder0000"),
//     };
//     let env = mock_env("asset0000", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     let msg = HandleMsg::TerraswapCreationHook {
//         asset_token: HumanAddr::from("asset0000"),
//     };
//     let env = mock_env("terraswapfactory", &[]);
//     let _res = handle(&mut deps, env, msg).unwrap();

//     // register queriers
//     deps.querier.with_mint_configs(&[(
//         &HumanAddr::from("asset0000"),
//         &(Decimal::percent(1), Decimal::percent(1), None),
//     )]);
//     deps.querier.with_oracle_feeders(&[(
//         &HumanAddr::from("asset0000"),
//         &HumanAddr::from("feeder0000"),
//     )]);

//     // unauthorized migrate attempt
//     let msg = HandleMsg::MigrateAsset {
//         name: "apple migration".to_string(),
//         symbol: "mAAPL2".to_string(),
//         from_token: HumanAddr::from("asset0000"),
//         end_price: Decimal::from_ratio(2u128, 1u128),
//     };
//     let env = mock_env("owner0000", &[]);
//     let res = handle(&mut deps, env, msg.clone()).unwrap_err();

//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let env = mock_env("feeder0000", &[]);
//     let res = handle(&mut deps, env, msg).unwrap();
//     assert_eq!(
//         res.messages,
//         vec![
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: HumanAddr::from("mint0000"),
//                 send: vec![],
//                 msg: to_binary(&MintHandleMsg::RegisterMigration {
//                     asset_token: HumanAddr::from("asset0000"),
//                     end_price: Decimal::from_ratio(2u128, 1u128),
//                 })
//                 .unwrap(),
//             }),
//             CosmosMsg::Wasm(WasmMsg::Instantiate {
//                 code_id: TOKEN_CODE_ID,
//                 send: vec![],
//                 label: None,
//                 msg: to_binary(&TokenInitMsg {
//                     name: "apple migration".to_string(),
//                     symbol: "mAAPL2".to_string(),
//                     decimals: 6u8,
//                     initial_balances: vec![],
//                     mint: Some(MinterResponse {
//                         minter: HumanAddr::from("mint0000"),
//                         cap: None,
//                     }),
//                     init_hook: Some(InitHook {
//                         contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
//                         msg: to_binary(&HandleMsg::TokenCreationHook {
//                             oracle_feeder: HumanAddr::from("feeder0000")
//                         })
//                         .unwrap(),
//                     }),
//                 })
//                 .unwrap(),
//             })
//         ]
//     );
// }
