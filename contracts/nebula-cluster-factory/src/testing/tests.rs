use crate::contract::{execute, instantiate, migrate, query, reply};
use crate::error::ContractError;
use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    cluster_exists, read_params, read_tmp_asset, read_tmp_cluster, read_total_weight, read_weight,
    store_total_weight, store_weight,
};
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::{Asset, AssetInfo};
use astroport::factory::{ExecuteMsg as AstroportFactoryExecuteMsg, PairType};
use astroport::token::InstantiateMsg as TokenInstantiateMsg;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Binary, ContractResult, CosmosMsg, Env, Reply, ReplyOn,
    StdError, SubMsg, SubMsgExecutionResponse, Timestamp, Uint128, WasmMsg,
};
use cw2::{get_contract_version, ContractVersion};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use nebula_protocol::cluster::{
    ExecuteMsg as ClusterExecuteMsg, InstantiateMsg as ClusterInstantiateMsg,
};
use nebula_protocol::cluster_factory::{
    ClusterExistsResponse, ClusterListResponse, ConfigResponse, DistributionInfoResponse,
    ExecuteMsg, InstantiateMsg, MigrateMsg, Params, QueryMsg,
};
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
use nebula_protocol::staking::{
    Cw20HookMsg as StakingCw20HookMsg, ExecuteMsg as StakingExecuteMsg,
};
use protobuf::Message;

fn mock_env_time(time: u64) -> Env {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(time);
    env
}

/// Convenience function for creating inline String
pub fn h(s: &str) -> String {
    s.to_string()
}

pub fn get_input_params() -> Params {
    Params {
        name: "Test Cluster".to_string(),
        symbol: "TEST".to_string(),
        description: "Sample cluster for testing".to_string(),
        weight: Some(100u32),
        penalty: Addr::unchecked("penalty0000"),
        pricing_oracle: Addr::unchecked("pricing_oracle0000"),
        target_oracle: Addr::unchecked("comp_oracle0000"),
        target: vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mAAPL"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mGOOG"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mMSFT"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mNFLX"),
                },
                amount: Uint128::new(20),
            },
        ],
    }
}

static TOKEN_CODE_ID: u64 = 8u64;
static CLUSTER_CODE_ID: u64 = 1u64;
static BASE_DENOM: &str = "uusd";
static PROTOCOL_FEE_RATE: &str = "0.01";

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // cannot update mirror token after initialization
    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0000".to_string(),
            nebula_token: "nebula0000".to_string(),
            staking_contract: "staking0000".to_string(),
            commission_collector: "collector0000".to_string(),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            astroport_factory: "astroportfactory".to_string(),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID,
            cluster_code_id: CLUSTER_CODE_ID,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![],
        }
    );

    // return false as there is no cluster registered yet
    assert_eq!(
        cluster_exists(&deps.storage, &Addr::unchecked("asset0000")).unwrap(),
        false
    );
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // update owner
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        distribution_schedule: None,
        token_code_id: None,
        cluster_code_id: None,
    };

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            nebula_token: "nebula0000".to_string(),
            staking_contract: "staking0000".to_string(),
            commission_collector: "collector0000".to_string(),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            astroport_factory: "astroportfactory".to_string(),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID,
            cluster_code_id: CLUSTER_CODE_ID,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![],
        }
    );

    // update rest part
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        distribution_schedule: Some(vec![(1, 2, Uint128::from(123u128))]),
        token_code_id: Some(TOKEN_CODE_ID + 1),
        cluster_code_id: Some(CLUSTER_CODE_ID + 1),
    };

    let info = mock_info("owner0001", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            nebula_token: "nebula0000".to_string(),
            staking_contract: "staking0000".to_string(),
            commission_collector: "collector0000".to_string(),
            protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
            astroport_factory: "astroportfactory".to_string(),
            base_denom: BASE_DENOM.to_string(),
            token_code_id: TOKEN_CODE_ID + 1,
            cluster_code_id: CLUSTER_CODE_ID + 1,
            genesis_time: 1_571_797_419,
            distribution_schedule: vec![(1, 2, Uint128::from(123u128))],
        }
    );

    // failed unauthorized
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        distribution_schedule: None,
        token_code_id: Some(TOKEN_CODE_ID + 1),
        cluster_code_id: Some(CLUSTER_CODE_ID + 1),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn test_update_weight() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);
    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    store_total_weight(deps.as_mut().storage, 100).unwrap();
    store_weight(deps.as_mut().storage, &Addr::unchecked("asset0000"), 10).unwrap();

    // increase weight
    let msg = ExecuteMsg::UpdateWeight {
        asset_token: h("asset0000"),
        weight: 20,
    };
    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let res = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(h("asset0000"), 20), (h("nebula0000"), 30)],
            last_distributed: 1_571_797_419,
        }
    );

    assert_eq!(
        read_weight(deps.as_mut().storage, &Addr::unchecked("asset0000")).unwrap(),
        20u32
    );
    assert_eq!(read_total_weight(deps.as_mut().storage).unwrap(), 110u32);
}

#[test]
fn test_create_cluster() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let _res = read_params(&deps.storage).unwrap_err();

    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "create_cluster"),
            attr("symbol", "TEST"),
            attr("name", "Test Cluster")
        ]
    );

    // token creation msg should be returned
    assert_eq!(
        res.messages,
        vec![SubMsg {
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: CLUSTER_CODE_ID,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&ClusterInstantiateMsg {
                    name: input_params.name.clone(),
                    description: input_params.description.clone(),
                    owner: MOCK_CONTRACT_ADDR.to_string(),
                    factory: MOCK_CONTRACT_ADDR.to_string(),
                    pricing_oracle: input_params.pricing_oracle.to_string(),
                    target_oracle: input_params.target_oracle.to_string(),
                    penalty: input_params.penalty.to_string(),
                    cluster_token: None,
                    target: input_params.target.clone(),
                })
                .unwrap(),
            }
            .into(),
            gas_limit: None,
            id: 1,
            reply_on: ReplyOn::Success,
        }]
    );

    let params: Params = read_params(deps.as_mut().storage).unwrap();
    assert_eq!(params, input_params);

    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("A cluster registration process is in progress".to_string())
    );

    let info = mock_info("addr0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});
}

#[test]
fn test_token_creation_hook() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let _res = read_params(&deps.storage).unwrap_err();

    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    let cluster = read_tmp_cluster(&deps.storage).unwrap();
    assert_eq!(cluster, "asset0000");

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: input_params.penalty.to_string(),
                funds: vec![],
                msg: to_binary(&PenaltyExecuteMsg::UpdateConfig {
                    owner: Some(h("asset0000")),
                    penalty_params: None,
                })
                .unwrap(),
            })),
            SubMsg {
                msg: WasmMsg::Instantiate {
                    admin: None,
                    code_id: TOKEN_CODE_ID,
                    funds: vec![],
                    label: "".to_string(),
                    msg: to_binary(&TokenInstantiateMsg {
                        name: input_params.name.clone(),
                        symbol: input_params.symbol.clone(),
                        decimals: 6u8,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: h("asset0000"),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                }
                .into(),
                gas_limit: None,
                id: 2,
                reply_on: ReplyOn::Success,
            },
        ]
    );

    assert_eq!(res.attributes, vec![attr("cluster_addr", "asset0000")]);

    // query to see if the created cluster exists
    let msg = QueryMsg::ClusterExists {
        contract_addr: "asset0000".to_string(),
    };
    let res: ClusterExistsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res, ClusterExistsResponse { exists: true });

    // and we can also get a list of all clusters
    let msg = QueryMsg::ClusterList {};
    let res: ClusterListResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        ClusterListResponse {
            contract_infos: vec![("asset0000".to_string(), true)]
        }
    );
}

#[test]
fn test_set_cluster_token_hook() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let _res = read_params(&deps.storage).unwrap_err();

    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    let cluster = read_tmp_cluster(&deps.storage).unwrap();
    assert_eq!(cluster, "asset0000");

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0000".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();
    let asset = read_tmp_asset(&deps.storage).unwrap();
    assert_eq!(asset, "cluster_token0000");

    assert_eq!(
        res.messages,
        vec![
            //Set cluster token
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("asset0000"),
                funds: vec![],
                msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
                    owner: Some(h("owner0000")),
                    name: None,
                    description: None,
                    cluster_token: Some(h("cluster_token0000")),
                    pricing_oracle: None,
                    target_oracle: None,
                    penalty: None,
                    target: None,
                })
                .unwrap(),
            })),
            // set up astroport pair
            SubMsg {
                msg: WasmMsg::Execute {
                    contract_addr: h("astroportfactory"),
                    funds: vec![],
                    msg: to_binary(&AstroportFactoryExecuteMsg::CreatePair {
                        pair_type: PairType::Xyk {},
                        asset_infos: [
                            AssetInfo::NativeToken {
                                denom: BASE_DENOM.to_string(),
                            },
                            AssetInfo::Token {
                                contract_addr: Addr::unchecked("cluster_token0000"),
                            },
                        ],
                        init_params: None
                    })
                    .unwrap(),
                }
                .into(),
                gas_limit: None,
                id: 3,
                reply_on: ReplyOn::Success,
            }
        ]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_cluster_token"),
            attr("cluster", "asset0000"),
            attr("token", "cluster_token0000")
        ]
    );

    let _res = read_params(&deps.storage).unwrap_err();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(h("cluster_token0000"), 100), (h("nebula0000"), 30)],
            last_distributed: 1_571_797_419,
        }
    );
}

#[test]
fn test_set_cluster_token_hook_without_weight() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .with_astroport_pairs(&[(&"uusdnebula0000".to_string(), &"NEBLP0000".to_string())]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let _res = read_params(&deps.storage).unwrap_err();

    let mut input_params: Params = get_input_params();
    input_params.weight = None;
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    let cluster = read_tmp_cluster(&deps.storage).unwrap();
    assert_eq!(cluster, "asset0000");

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0000".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();
    let asset = read_tmp_asset(&deps.storage).unwrap();
    assert_eq!(asset, "cluster_token0000");

    assert_eq!(
        res.messages,
        vec![
            //Set cluster token
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("asset0000"),
                funds: vec![],
                msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
                    owner: Some(h("owner0000")),
                    name: None,
                    description: None,
                    cluster_token: Some(h("cluster_token0000")),
                    pricing_oracle: None,
                    target_oracle: None,
                    penalty: None,
                    target: None,
                })
                .unwrap(),
            })),
            // set up astroport pair
            SubMsg {
                msg: WasmMsg::Execute {
                    contract_addr: h("astroportfactory"),
                    funds: vec![],
                    msg: to_binary(&AstroportFactoryExecuteMsg::CreatePair {
                        pair_type: PairType::Xyk {},
                        asset_infos: [
                            AssetInfo::NativeToken {
                                denom: BASE_DENOM.to_string(),
                            },
                            AssetInfo::Token {
                                contract_addr: Addr::unchecked("cluster_token0000"),
                            },
                        ],
                        init_params: None
                    })
                    .unwrap(),
                }
                .into(),
                gas_limit: None,
                id: 3,
                reply_on: ReplyOn::Success,
            }
        ]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_cluster_token"),
            attr("cluster", "asset0000"),
            attr("token", "cluster_token0000")
        ]
    );

    let _res = read_params(&deps.storage).unwrap_err();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(h("cluster_token0000"), 30), (h("nebula0000"), 30)],
            last_distributed: 1_571_797_419,
        }
    );
}

#[test]
fn test_astroport_creation_hook() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_astroport_pairs(&[
        (&"uusdnebula0000".to_string(), &"NEBLP000".to_string()),
        (&"uusdcluster_token0000".to_string(), &"LP0000".to_string()),
    ]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };

    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let _res = read_params(&deps.storage).unwrap_err();

    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    let cluster = read_tmp_cluster(&deps.storage).unwrap();
    assert_eq!(cluster, "asset0000");

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0000".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();
    let asset = read_tmp_asset(&deps.storage).unwrap();
    assert_eq!(asset, "cluster_token0000");

    let reply_msg3 = Reply {
        id: 3,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let res = reply(deps.as_mut(), mock_env(), reply_msg3).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "staking0000".to_string(),
            funds: vec![],
            msg: to_binary(&StakingExecuteMsg::RegisterAsset {
                asset_token: h("cluster_token0000"),
                staking_token: h("LP0000"),
            })
            .unwrap(),
        }))]
    );
}

#[test]
fn test_distribute() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_astroport_pairs(&[
        (&"uusdnebula0000".to_string(), &"NEBLP000".to_string()),
        (&"uusdcluster_token0000".to_string(), &h("LP0000")),
        (&"uusdcluster_token0001".to_string(), &h("LP0001")),
    ]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![
            (1800, 3600, Uint128::from(3600u128)),
            (3600, 3600 + 3600, Uint128::from(7200u128)),
        ],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // create first cluster with weight 100
    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0000".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();

    let reply_msg3 = Reply {
        id: 3,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg3).unwrap();

    // create second cluster with weight 30
    let mut input_params: Params = get_input_params();
    input_params.weight = Some(30u32);
    input_params.name = "Test Cluster 2".to_string();
    input_params.symbol = "TEST2".to_string();

    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0001".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0001".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();

    let reply_msg3 = Reply {
        id: 3,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg3).unwrap();

    // height is not increased so zero amount will be minted
    let msg = ExecuteMsg::Distribute {};
    let info = mock_info("anyone", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("Cannot distribute nebula token before interval".to_string())
    );

    // one height increase
    let msg = ExecuteMsg::Distribute {};
    let env = mock_env_time(1_571_797_419u64 + 5400u64);
    let info = mock_info(&"addr0000".to_string(), &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "distribute"),
            attr("distribution_amount", "7200"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: h("nebula0000"),
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: h("staking0000"),
                amount: Uint128::new(7200u128),
                msg: to_binary(&StakingCw20HookMsg::DepositReward {
                    rewards: vec![
                        (h("cluster_token0000"), Uint128::new(4500)),
                        (h("cluster_token0001"), Uint128::new(1350)),
                        (h("nebula0000"), Uint128::new(1350)),
                    ],
                })
                .unwrap()
            })
            .unwrap(),
            funds: vec![],
        })],
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![
                (h("cluster_token0000"), 100),
                (h("cluster_token0001"), 30),
                (h("nebula0000"), 30)
            ],
            last_distributed: 1_571_802_819,
        }
    );
}

#[test]
fn test_decommission_cluster() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_astroport_pairs(&[
        (&"uusdnebula0000".to_string(), &"NEBLP000".to_string()),
        (&"uusdcluster_token0000".to_string(), &h("LP0000")),
        (&"uusdcluster_token0001".to_string(), &h("LP0001")),
    ]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![
            (1800, 3600, Uint128::from(3600u128)),
            (3600, 3600 + 3600, Uint128::from(7200u128)),
        ],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Create a test cluster
    let input_params: Params = get_input_params();
    let msg = ExecuteMsg::CreateCluster {
        params: input_params.clone(),
    };
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("asset0000".to_string());

    let reply_msg = Reply {
        id: 1,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    let cluster = read_tmp_cluster(&deps.storage).unwrap();
    assert_eq!(cluster, "asset0000");

    let mut token_inst_res = MsgInstantiateContractResponse::new();
    token_inst_res.set_contract_address("cluster_token0000".to_string());

    let reply_msg2 = Reply {
        id: 2,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: Some(token_inst_res.write_to_bytes().unwrap().into()),
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg2).unwrap();
    let asset = read_tmp_asset(&deps.storage).unwrap();
    assert_eq!(asset, "cluster_token0000");

    let reply_msg3 = Reply {
        id: 3,
        result: ContractResult::Ok(SubMsgExecutionResponse {
            events: vec![],
            data: None,
        }),
    };

    let _res = reply(deps.as_mut(), mock_env(), reply_msg3).unwrap();

    // unauthorized decomission attempt
    let msg = ExecuteMsg::DecommissionCluster {
        cluster_contract: h("asset0000"),
        cluster_token: h("cluster_token0000"),
    };
    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();

    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: h("asset0000"),
            funds: vec![],
            msg: to_binary(&ClusterExecuteMsg::Decommission {}).unwrap(),
        }))]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "decommission_asset"),
            attr("cluster_token", "cluster_token0000"),
            attr("cluster_contract", "asset0000"),
        ]
    );

    assert_eq!(
        cluster_exists(&deps.storage, &Addr::unchecked("asset0000")).unwrap(),
        false
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::DistributionInfo {}).unwrap();
    let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();

    assert_eq!(
        distribution_info,
        DistributionInfoResponse {
            weights: vec![(h("nebula0000"), 30)],
            last_distributed: 1_571_797_419,
        }
    );

    let res = read_weight(&deps.storage, &Addr::unchecked("asset0000")).unwrap_err();
    assert_eq!(res, StdError::generic_err("No distribution info stored"));

    assert_eq!(read_total_weight(&deps.storage).unwrap(), 30u32);
}

#[test]
fn test_pass_command() {
    let mut deps = mock_dependencies(&[]);
    deps.querier.with_astroport_pairs(&[
        (&"uusdnebula0000".to_string(), &"NEBLP000".to_string()),
        (&"uusdcluster_token0000".to_string(), &h("LP0000")),
        (&"uusdcluster_token0001".to_string(), &h("LP0001")),
    ]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![
            (1800, 3600, Uint128::from(3600u128)),
            (3600, 3600 + 3600, Uint128::from(7200u128)),
        ],
    };

    let info = mock_info("addr0000", &[]);

    // we can call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::PostInitialize {
        owner: "owner0000".to_string(),
        nebula_token: "nebula0000".to_string(),
        staking_contract: "staking0000".to_string(),
        commission_collector: "collector0000".to_string(),
        astroport_factory: "astroportfactory".to_string(),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // failed non-owner call
    let msg = ExecuteMsg::PassCommand {
        contract_addr: "contract0001".to_string(),
        msg: Binary::default(),
    };

    let info = mock_info("imposter0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // successfully pass command
    let msg = ExecuteMsg::PassCommand {
        contract_addr: "contract0001".to_string(),
        msg: Binary::default(),
    };

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
}

#[test]
fn migration() {
    let mut deps = mock_dependencies(&[]);

    let msg = InstantiateMsg {
        base_denom: BASE_DENOM.to_string(),
        token_code_id: TOKEN_CODE_ID,
        cluster_code_id: CLUSTER_CODE_ID,
        protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
        distribution_schedule: vec![],
    };

    let info = mock_info("addr0000", &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-cluster-factory".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
