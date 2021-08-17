// use crate::contract::{execute, instantiate, query};
// use crate::mock_querier::mock_dependencies;

// use crate::state::{
//     cluster_exists, read_params, read_total_weight, read_weight, store_total_weight, store_weight,
// };
// use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
// use cosmwasm_std::{
//     attr, from_binary, to_binary, CosmosMsg, Env, StdError, Timestamp, Uint128, WasmMsg,
// };
// use cw20::{Cw20ExecuteMsg, MinterResponse};

// use nebula_protocol::cluster_factory::{
//     ConfigResponse, DistributionInfoResponse, ExecuteMsg, InstantiateMsg, Params, QueryMsg,
// };

// use nebula_protocol::cluster::{
//     ExecuteMsg as ClusterExecuteMsg, InstantiateMsg as ClusterInstantiateMsg,
// };
// use nebula_protocol::oracle::ExecuteMsg as OracleExecuteMsg;
// use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
// use nebula_protocol::staking::{
//     Cw20HookMsg as StakingCw20HookMsg, ExecuteMsg as StakingExecuteMsg,
// };
// use terraswap::asset::{Asset, AssetInfo};
// use terraswap::factory::ExecuteMsg as TerraswapFactoryExecuteMsg;
// use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

// fn mock_info_time(signer: &String, time: u64) -> Env {
//     let mut env = mock_info(signer, &[]);
//     env.block.time = Timestamp::from_seconds(time);
//     env
// }

// /// Convenience function for creating inline String
// pub fn h(s: &str) -> String {
//     s.to_string()
// }

// pub fn get_input_params() -> Params {
//     Params {
//         name: "Test Cluster".to_string(),
//         symbol: "TEST".to_string(),
//         description: "Sample cluster for testing".to_string(),
//         weight: Some(100u32),
//         penalty: ("penalty0000"),
//         pricing_oracle: ("pricing_oracle0000"),
//         composition_oracle: ("comp_oracle0000"),
//         target: vec![
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: h("mAAPL"),
//                 },
//                 amount: Uint128::new(20),
//             },
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: h("mGOOG"),
//                 },
//                 amount: Uint128::new(20),
//             },
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: h("mMSFT"),
//                 },
//                 amount: Uint128::new(20),
//             },
//             Asset {
//                 info: AssetInfo::Token {
//                     contract_addr: h("mNFLX"),
//                 },
//                 amount: Uint128::new(20),
//             },
//         ],
//     }
// }

// static TOKEN_CODE_ID: u64 = 8u64;
// static CLUSTER_CODE_ID: u64 = 1u64;
// static BASE_DENOM: &str = "uusd";
// static PROTOCOL_FEE_RATE: &str = "0.01";

// #[test]
// fn proper_initialization() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         terraswap_factory: ("terraswapfactory"),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//     };
//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     // cannot update mirror token after initialization
//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         terraswap_factory: ("terraswapfactory"),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//     };
//     let _res = execute(deps.as_mut(), env, msg).unwrap_err();

//     // it worked, let's query the state
//     let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
//     let config: ConfigResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         config,
//         ConfigResponse {
//             owner: "owner0000".to_string(),
//             nebula_token:"nebula0000".to_string(),
//             staking_contract: ("staking0000"),
//             commission_collector: ("collector0000"),
//             protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//             terraswap_factory: ("terraswapfactory"),
//             base_denom: BASE_DENOM.to_string(),
//             token_code_id: TOKEN_CODE_ID,
//             cluster_code_id: CLUSTER_CODE_ID,
//             genesis_time: 1_571_797_419,
//             distribution_schedule: vec![],
//         }
//     );
// }

// #[test]
// fn test_update_config() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };
//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     // upate owner
//     let msg = ExecuteMsg::UpdateConfig {
//         owner: Some(("owner0001")),
//         distribution_schedule: None,
//         token_code_id: None,
//         cluster_code_id: None,
//     };

//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();
//     let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
//     let config: ConfigResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         config,
//         ConfigResponse {
//             owner:"owner0001".to_string(),
//             nebula_token:"nebula0000".to_string(),
//             staking_contract: ("staking0000"),
//             commission_collector: ("collector0000"),
//             protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//             terraswap_factory: ("terraswapfactory"),
//             base_denom: BASE_DENOM.to_string(),
//             token_code_id: TOKEN_CODE_ID,
//             cluster_code_id: CLUSTER_CODE_ID,
//             genesis_time: 1_571_797_419,
//             distribution_schedule: vec![],
//         }
//     );

//     // update rest part
//     let msg = ExecuteMsg::UpdateConfig {
//         owner: None,
//         distribution_schedule: Some(vec![(1, 2, Uint128::from(123u128))]),
//         token_code_id: Some(TOKEN_CODE_ID + 1),
//         cluster_code_id: Some(CLUSTER_CODE_ID + 1),
//     };

//     let env = mock_info("owner0001", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();
//     let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
//     let config: ConfigResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         config,
//         ConfigResponse {
//             owner:"owner0001".to_string(),
//             nebula_token:"nebula0000".to_string(),
//             staking_contract: ("staking0000"),
//             commission_collector: ("collector0000"),
//             protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//             terraswap_factory: ("terraswapfactory"),
//             base_denom: BASE_DENOM.to_string(),
//             token_code_id: TOKEN_CODE_ID + 1,
//             cluster_code_id: CLUSTER_CODE_ID + 1,
//             genesis_time: 1_571_797_419,
//             distribution_schedule: vec![(1, 2, Uint128::from(123u128))],
//         }
//     );

//     // failed unauthoirzed
//     let msg = ExecuteMsg::UpdateConfig {
//         owner: None,
//         distribution_schedule: None,
//         token_code_id: Some(TOKEN_CODE_ID + 1),
//         cluster_code_id: Some(CLUSTER_CODE_ID + 1),
//     };

//     let env = mock_info("owner0000", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap_err();
//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn test_update_weight() {
//     let mut deps = mock_dependencies(20, &[]);
//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };
//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     store_total_weight(deps.storage, 100).unwrap();
//     store_weight(deps.storage, &("asset0000"), 10).unwrap();

//     // incrase weight
//     let msg = ExecuteMsg::UpdateWeight {
//         asset_token: ("asset0000"),
//         weight: 20,
//     };
//     let env = mock_info("owner0001", &[]);
//     let res = execute(deps.as_mut(), env, msg.clone()).unwrap_err();
//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();
//     let res = query(deps.as_ref(), QueryMsg::DistributionInfo {}).unwrap();
//     let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         distribution_info,
//         DistributionInfoResponse {
//             weights: vec![(("asset0000"), 20)],
//             last_distributed: 1_571_797_419,
//         }
//     );

//     assert_eq!(read_weight(deps.storage, &("asset0000")).unwrap(), 20u32);
//     assert_eq!(read_total_weight(deps.storage).unwrap(), 110u32);
// }

// #[test]
// fn test_create_cluster() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };

//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "create_cluster"),
//             attr("symbol", "TEST"),
//             attr("name", "Test Cluster")
//         ]
//     );

//     // token creation msg should be returned
//     assert_eq!(
//         res.messages,
//         vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
//             code_id: CLUSTER_CODE_ID,
//             funds: vec![],
//             label: None,
//             msg: to_binary(&ClusterInstantiateMsg {
//                 name: input_params.name.clone(),
//                 description: input_params.description.clone(),
//                 owner: hMOCK_CONTRACT_ADDR.to_string(),
//                 factory: hMOCK_CONTRACT_ADDR.to_string(),
//                 pricing_oracle: input_params.pricing_oracle.clone(),
//                 composition_oracle: input_params.composition_oracle.clone(),
//                 penalty: input_params.penalty.clone(),
//                 cluster_token: None,
//                 target: input_params.target.clone(),
//                 // init_hook: Some(InitHook {
//                 //     contract_addr: MOCK_CONTRACT_ADDR.to_string(),
//                 //     msg: to_binary(&ExecuteMsg::TokenCreationHook {}).unwrap(),
//                 // }),
//             })
//             .unwrap(),
//         })]
//     );

//     let params: Params = read_params(deps.storage).unwrap();
//     assert_eq!(params, input_params);

//     let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap_err();
//     match res {
//         StdError::GenericErr { msg, .. } => {
//             assert_eq!(msg, "A cluster registration process is in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let env = mock_info("addr0001", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap_err();
//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn test_token_creation_hook() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };

//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     // There is no cluster registration process; failed
//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone());
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "No cluster registration process in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};

//     let env = mock_info("asset0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();
//     assert_eq!(
//         res.messages,
//         vec![
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: input_params.penalty.clone(),
//                 funds: vec![],
//                 msg: to_binary(&PenaltyExecuteMsg::UpdateConfig {
//                     owner: Some(h("asset0000")),
//                     penalty_params: None,
//                 })
//                 .unwrap(),
//             }),
//             CosmosMsg::Wasm(WasmMsg::Instantiate {
//                 code_id: TOKEN_CODE_ID,
//                 funds: vec![],
//                 label: None,
//                 msg: to_binary(&TokenInstantiateMsg {
//                     name: input_params.name.clone(),
//                     symbol: input_params.symbol.clone(),
//                     decimals: 6u8,
//                     initial_balances: vec![],
//                     mint: Some(MinterResponse {
//                         minter: h("asset0000"),
//                         cap: None,
//                     }),
//                     // Set Cluster Token
//                     // init_hook: Some(InitHook {
//                     //     contract_addr: hMOCK_CONTRACT_ADDR.to_string(),
//                     //     msg: to_binary(&ExecuteMsg::SetClusterTokenHook {
//                     //         cluster: h("asset0000"),
//                     //     })
//                     //     .unwrap(),
//                     // }),
//                 })
//                 .unwrap(),
//             }),
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: h("asset0000"),
//                 funds: vec![],
//                 msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
//                     owner: Some("owner0000".to_string()),
//                     name: None,
//                     description: None,
//                     cluster_token: None,
//                     pricing_oracle: None,
//                     composition_oracle: None,
//                     penalty: None,
//                     target: None,
//                 })
//                 .unwrap(),
//             }),
//         ]
//     );

//     assert_eq!(res.attributes, vec![attr("cluster_addr", "asset0000")]);

//     assert_eq!(cluster_exists(deps.storage, &h("asset0000")), Ok(true))
// }

// #[test]
// fn test_set_cluster_token_hook() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };

//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     // There is no cluster registration process; failed
//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };
//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone());
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "No cluster registration process in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };

//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap();

//     assert_eq!(
//         res.messages,
//         vec![
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: h("asset0000"),
//                 funds: vec![],
//                 msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
//                     owner: None,
//                     name: None,
//                     description: None,
//                     cluster_token: Some(h("cluster_token0000")),
//                     pricing_oracle: None,
//                     composition_oracle: None,
//                     penalty: None,
//                     target: None,
//                 })
//                 .unwrap(),
//             }),
//             // set up terraswap pair
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: h("terraswapfactory"),
//                 funds: vec![],
//                 msg: to_binary(&TerraswapFactoryExecuteMsg::CreatePair {
//                     asset_infos: [
//                         AssetInfo::NativeToken {
//                             denom: BASE_DENOM.to_string(),
//                         },
//                         AssetInfo::Token {
//                             contract_addr: h("cluster_token0000"),
//                         },
//                     ],
//                     // init_hook: Some(InitHook {
//                     //     msg: to_binary(&ExecuteMsg::TerraswapCreationHook {
//                     //         asset_token: h("cluster_token0000"),
//                     //     })
//                     //     .unwrap(),
//                     //     contract_addr: hMOCK_CONTRACT_ADDR.to_string(),
//                     // }),
//                 })
//                 .unwrap(),
//             })
//         ]
//     );

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "set_cluster_token"),
//             attr("cluster", "asset0000"),
//             attr("token", "cluster_token0000")
//         ]
//     );

//     let res = query(deps.as_ref(), QueryMsg::DistributionInfo {}).unwrap();
//     let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         distribution_info,
//         DistributionInfoResponse {
//             weights: vec![(("cluster_token0000"), 100)],
//             last_distributed: 1_571_797_419,
//         }
//     );

//     // After execution of these hook, params is removed so we check that
//     // there is no cluster registration process; failed
//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };
//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone());
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "No cluster registration process in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn test_set_cluster_token_hook_without_weight() {
//     let mut deps = mock_dependencies(20, &[]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };

//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     // There is no cluster registration process; failed
//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };
//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone());
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "No cluster registration process in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let mut input_params: Params = get_input_params();
//     input_params.weight = None;
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };

//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap();

//     assert_eq!(
//         res.messages,
//         vec![
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: h("asset0000"),
//                 funds: vec![],
//                 msg: to_binary(&ClusterExecuteMsg::UpdateConfig {
//                     owner: None,
//                     name: None,
//                     description: None,
//                     cluster_token: Some(h("cluster_token0000")),
//                     pricing_oracle: None,
//                     composition_oracle: None,
//                     penalty: None,
//                     target: None,
//                 })
//                 .unwrap(),
//             }),
//             // set up terraswap pair
//             CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: h("terraswapfactory"),
//                 funds: vec![],
//                 msg: to_binary(&TerraswapFactoryExecuteMsg::CreatePair {
//                     asset_infos: [
//                         AssetInfo::NativeToken {
//                             denom: BASE_DENOM.to_string(),
//                         },
//                         AssetInfo::Token {
//                             contract_addr: h("cluster_token0000"),
//                         },
//                     ],
//                     // init_hook: Some(InitHook {
//                     //     msg: to_binary(&ExecuteMsg::TerraswapCreationHook {
//                     //         asset_token: h("cluster_token0000"),
//                     //     })
//                     //     .unwrap(),
//                     //     contract_addr: hMOCK_CONTRACT_ADDR.to_string(),
//                     // }),
//                 })
//                 .unwrap(),
//             })
//         ]
//     );

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "set_cluster_token"),
//             attr("cluster", "asset0000"),
//             attr("token", "cluster_token0000")
//         ]
//     );

//     let res = query(deps.as_ref(), QueryMsg::DistributionInfo {}).unwrap();
//     let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         distribution_info,
//         DistributionInfoResponse {
//             weights: vec![(("cluster_token0000"), 30)],
//             last_distributed: 1_571_797_419,
//         }
//     );

//     // After execution of these hook, params is removed so we check that
//     // there is no cluster registration process; failed
//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };
//     let env = mock_info("cluster_token0000", &[]);
//     let res = execute(deps.as_mut(), env.clone(), msg.clone());
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "No cluster registration process in progress")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }
// }

// #[test]
// fn test_terraswap_creation_hook() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier
//         .with_terraswap_pairs(&[(&"uusdasset0000".to_string(), &("LP0000"))]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };

//     let _res = execute(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::TerraswapCreationHook {
//         asset_token: ("asset0000"),
//     };

//     let env = mock_info("terraswapfactory1", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap_err();

//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };

//     let env = mock_info("cluster_token0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::TerraswapCreationHook {
//         asset_token: ("asset0000"),
//     };

//     let env = mock_info("terraswapfactory", &[]);
//     let res = execute(deps.as_mut(), env, msg).unwrap();

//     assert_eq!(
//         res.messages,
//         vec![CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: ("staking0000"),
//             funds: vec![],
//             msg: to_binary(&StakingExecuteMsg::RegisterAsset {
//                 asset_token: ("asset0000"),
//                 staking_token: ("LP0000"),
//             })
//             .unwrap(),
//         })]
//     );
// }

// #[test]
// fn test_distribute() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier.with_terraswap_pairs(&[
//         (&"uusdasset0000".to_string(), &("LP0000")),
//         (&"uusdasset0001".to_string(), &("LP0001")),
//     ]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![
//             (1800, 3600, Uint128::from(3600u128)),
//             (3600, 3600 + 3600, Uint128::from(7200u128)),
//         ],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     // create first cluter with weight 100
//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };

//     let env = mock_info("cluster_token0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::TerraswapCreationHook {
//         asset_token: ("asset0000"),
//     };

//     let env = mock_info("terraswapfactory", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     // create second cluter with weight 30
//     let mut input_params: Params = get_input_params();
//     input_params.weight = Some(30u32);
//     input_params.name = "Test Cluster 2".to_string();
//     input_params.symbol = "TEST2".to_string();

//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0001", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0001"),
//     };

//     let env = mock_info("cluster_token0001", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::TerraswapCreationHook {
//         asset_token: ("asset0001"),
//     };

//     let env = mock_info("terraswapfactory", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     // height is not increased so zero amount will be minted
//     let msg = ExecuteMsg::Distribute {};
//     let env = mock_info("anyone", &[]);
//     let res = execute(deps.as_mut(), env, msg);
//     match res {
//         Err(StdError::GenericErr { msg, .. }) => {
//             assert_eq!(msg, "Cannot distribute nebula token before interval")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     // one height increase
//     let msg = ExecuteMsg::Distribute {};
//     let env = mock_info_time(&"addr0000".to_string(), 1_571_797_419u64 + 5400u64);
//     let res = execute(deps.as_mut(), env, msg).unwrap();
//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "distribute"),
//             attr("distribution_amount", "7199"),
//         ]
//     );

//     assert_eq!(
//         res.messages,
//         vec![CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: h("nebula0000"),
//             msg: to_binary(&Cw20ExecuteMsg::Send {
//                 contract: h("staking0000"),
//                 amount: Uint128::new(7199u128),
//                 (msg:
//                     to_binary(&StakingCw20HookMsg::DepositReward {
//                         rewards: vec![
//                             (("cluster_token0000"), Uint128::new(5538)),
//                             (("cluster_token0001"), Uint128::new(1661)),
//                         ],
//                     })
//                     .unwrap()
//             })
//             .unwrap(),
//             funds: vec![],
//         }),],
//     );

//     let res = query(deps.as_ref(), QueryMsg::DistributionInfo {}).unwrap();
//     let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();
//     assert_eq!(
//         distribution_info,
//         DistributionInfoResponse {
//             weights: vec![(("cluster_token0000"), 100), (("cluster_token0001"), 30)],
//             last_distributed: 1_571_802_819,
//         }
//     );
// }

// #[test]
// fn test_decommission_cluster() {
//     let mut deps = mock_dependencies(20, &[]);
//     deps.querier
//         .with_terraswap_pairs(&[(&"uusdasset0000".to_string(), &("LP0000"))]);

//     let msg = InstantiateMsg {
//         base_denom: BASE_DENOM.to_string(),
//         token_code_id: TOKEN_CODE_ID,
//         cluster_code_id: CLUSTER_CODE_ID,
//         protocol_fee_rate: PROTOCOL_FEE_RATE.to_string(),
//         distribution_schedule: vec![
//             (1800, 3600, Uint128::from(3600u128)),
//             (3600, 3600 + 3600, Uint128::from(7200u128)),
//         ],
//     };

//     let env = mock_info("addr0000", &[]);
//     let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

//     let msg = ExecuteMsg::PostInitialize {
//         owner: "owner0000".to_string(),
//         nebula_token:"nebula0000".to_string(),
//         staking_contract: ("staking0000"),
//         commission_collector: ("collector0000"),
//         terraswap_factory: ("terraswapfactory"),
//     };
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     // Create a test cluster
//     let input_params: Params = get_input_params();
//     let msg = ExecuteMsg::CreateCluster {
//         params: input_params.clone(),
//     };
//     let env = mock_info("owner0000", &[]);
//     let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

//     let msg = ExecuteMsg::TokenCreationHook {};
//     let env = mock_info("asset0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::SetClusterTokenHook {
//         cluster: h("asset0000"),
//     };

//     let env = mock_info("cluster_token0000", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     let msg = ExecuteMsg::TerraswapCreationHook {
//         asset_token: ("asset0000"),
//     };

//     let env = mock_info("terraswapfactory", &[]);
//     let _res = execute(deps.as_mut(), env, msg).unwrap();

//     // unauthorized decomission attempt
//     let msg = ExecuteMsg::DecommissionCluster {
//         cluster_contract: h("asset0000"),
//         cluster_token: h("cluster_token0000"),
//     };
//     let env = mock_info("owner0001", &[]);
//     let res = execute(deps.as_mut(), env, msg.clone()).unwrap_err();

//     match res {
//         StdError::Unauthorized { .. } => {}
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     let env = mock_info("owner0000", &[]);
//     let res = execute(deps.as_mut(), env, msg.clone()).unwrap();

//     assert_eq!(
//         res.messages,
//         vec![CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: h("asset0000"),
//             funds: vec![],
//             msg: to_binary(&ClusterExecuteMsg::Decommission {}).unwrap(),
//         })]
//     );

//     assert_eq!(
//         res.attributes,
//         vec![
//             attr("action", "decommission_asset"),
//             attr("cluster_token", "cluster_token0000"),
//             attr("cluster_contract", "asset0000"),
//         ]
//     );

//     assert_eq!(
//         cluster_exists(deps.storage, &h("asset0000")).unwrap(),
//         false
//     );

//     let res = query(deps.as_ref(), QueryMsg::DistributionInfo {}).unwrap();
//     let distribution_info: DistributionInfoResponse = from_binary(&res).unwrap();

//     assert_eq!(
//         distribution_info,
//         DistributionInfoResponse {
//             weights: vec![],
//             last_distributed: 1_571_797_419,
//         }
//     );

//     let res = read_weight(deps.storage, &("asset0000")).unwrap_err();
//     match res {
//         StdError::GenericErr { msg, .. } => {
//             assert_eq!(msg, "No distribution info stored")
//         }
//         _ => panic!("DO NOT ENTER HERE"),
//     }

//     assert_eq!(read_total_weight(deps.storage).unwrap(), 0u32);
// }
