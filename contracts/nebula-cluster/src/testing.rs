pub use crate::contract::*;
pub use crate::ext_query::*;
use crate::mock_querier::consts;
use crate::mock_querier::mock_dependencies;
use crate::mock_querier::mock_init;
use crate::mock_querier::mock_querier_setup;
use crate::mock_querier::token_data;
pub use crate::state::*;
pub use cluster_math::*;
pub use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
pub use cosmwasm_std::*;
pub use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20::{Cw20HandleMsg};
use nebula_protocol::{
    cluster::{HandleMsg, InitMsg, QueryMsg as ClusterQueryMsg, TargetResponse},
    cluster_factory::ConfigResponse as FactoryConfigResponse,
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    penalty::{MintResponse, QueryMsg as PenaltyQueryMsg, RedeemResponse},
};
use nebula_protocol::penalty::{HandleMsg as PenaltyHandleMsg};
use pretty_assertions::assert_eq;
use std::collections::HashMap;
pub use std::str::FromStr;
use terra_cosmwasm::*;
use terraswap::asset::{Asset, AssetInfo};
pub use crate::mock_querier;

#[macro_export]
macro_rules! q {
    ($deps:expr, $val_type:ty, $msg: expr) => {{
        let res = query($deps, $msg).unwrap();
        let val: $val_type = from_binary(&res).unwrap();
        val
    }};
}

#[test]
fn proper_initialization() {
    let (deps, init_res) = mock_init();
    assert_eq!(0, init_res.messages.len());
    
    // make sure target was saved
    let value = q!(&deps, TargetResponse, ClusterQueryMsg::Target {});
    assert_eq!(
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128(20)
            },
        ],
        value.target
    );
}

#[test]
fn fail_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: consts::target_assets(),
        pricing_oracle: consts::pricing_oracle(),
        composition_oracle: consts::composition_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
        init_hook: None,
    };

    let env = mock_env(consts::pricing_oracle().as_str(), &[]);
    let res = init(&mut deps, env.clone(), msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cluster must contain valid assets and cannot contain duplicate assets"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }
}

#[test]
fn mint() {
    let (mut deps, _) = mock_init();
    // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
    // ---
    // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
    // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
    // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
    // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389

    // The set token balance should include the amount we would also like to stage
    deps.querier
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "mint"),
            log("sender", "addr0000"),
            log("mint_to_sender", "98"),
            log("penalty", "1234"),
            log("fee_amt", "1"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("mAAPL"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: HumanAddr::from("addr0000"),
                    recipient: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128(125_000_000),
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("mGOOG"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: HumanAddr::from("addr0000"),
                    recipient: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128::zero(),
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("mMSFT"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: HumanAddr::from("addr0000"),
                    recipient: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128(149_000_000),
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("mNFLX"),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: HumanAddr::from("addr0000"),
                    recipient: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    amount: Uint128(50_090_272),
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyHandleMsg::Mint {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128(1_000_000_000),
                    inventory: vec![
                        Uint128(7_290_053_159u128), Uint128(319_710_128u128),
                        Uint128(14_219_281_228u128), Uint128(224_212_221u128)
                    ],
                    mint_asset_amounts: vec![
                        Uint128(125_000_000),
                        Uint128::zero(),
                        Uint128(149_000_000),
                        Uint128(50_090_272),
                    ],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string()
                    ],
                    target_weights: vec![
                        Uint128(20u128),
                        Uint128(20u128),
                        Uint128(20u128),
                        Uint128(20u128)
                    ],
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20HandleMsg::Mint {
                    amount: Uint128(1u128),
                    recipient: h("collector"),
                }).unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20HandleMsg::Mint {
                    amount: Uint128(98),
                    recipient: HumanAddr::from("addr0000"),
                }).unwrap(),
                send: vec![],
            })
        ]
        
    );

    assert_eq!(7, res.messages.len());
}

#[test]
fn fail_mint_target_zero() {
    let mut deps = mock_dependencies(20, &[]);
    mock_querier_setup(&mut deps);

    let msg = InitMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
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
                amount: Uint128(0),
            },
        ],
        pricing_oracle: consts::pricing_oracle(),
        composition_oracle: consts::composition_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
        init_hook: None,
    };

    let env = mock_env(consts::pricing_oracle().as_str(), &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    deps.querier
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cannot mint with non-zero asset amount when target weight is zero for asset mNFLX"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }
}

#[test]
fn fail_mint_unsupported_coin() {
    let (mut deps, _) = mock_init();
    deps.querier
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(149_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(50_090_272),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "UST".to_string(),
            },
            amount: Uint128(50_090_272),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &coins(50_090_272, &"UST".to_string()));
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Unsupported assets were sent to the mint function"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }
}

#[test]
fn test_initial_mint_with_fails() {
    let (mut deps, _) = mock_init();
    deps.querier
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ])
        .set_token(
            consts::cluster_token(),
            token_data::<Vec<(&str, u128)>, &str>(
                "Cluster Token",
                "CLUSTER",
                6,
                0,
                vec![],
            ),
        );

    // Test initial mint without min_tokens field
    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(149_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(50_090_272),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Cluster is uninitialized. To initialize it with your mint cluster, provide min_tokens as the amount of cluster tokens you want to start with."),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }

    // Test with zero asset amount (will prompt divide by 0 panic if not caught)
    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128::zero(),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(149_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(50_090_272),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: Some(Uint128(1_000_000)),
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Initial cluster assets must be a nonzero multiple of target weights at index 1"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }

    // Test with incorrect ratio asset amount
    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(50_090_272),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: Some(Uint128(1_000_000)),
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Initial cluster assets must be a nonzero multiple of target weights at index 3"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }

    // Test with incorrect ratio asset amount again
    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(50_090_200),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: Some(Uint128(1_000_000)),
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg);

    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Initial cluster assets must be a multiple of target weights at index 3"),
        Err(e) => panic!("Unexpected error: {:?}", e),
        _ => panic!("Must return error"),
    }

    // Succeeds as a normal mint
    let asset_amounts = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(125_000_000),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            amount: Uint128(125_000_000),
        },
    ];

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: Some(Uint128(1_000_000)),
    };

    let addr = "addr0000";
    let env = mock_env(h(addr), &[]);
    let res = handle(&mut deps, env.clone(), mint_msg).unwrap();
    assert_eq!(5, res.messages.len());
}

#[test]
fn burn() {
    let (mut deps, _init_res) = mock_init();

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000)
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let msg = HandleMsg::Burn {
        max_tokens: Uint128(20_000_000),
        asset_amounts: None,
    };
    let env = mock_env(h("addr0000"), &[]);
    let res = handle(&mut deps, env.clone(), msg).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "receive:burn"),
            log("sender", "addr0000"),
            log("burn_amount", "1234"),
            log("token_cost", "1247"),
            log("kept_as_fee", "13"),
            log("asset_amounts", "[]"),
            log("redeem_totals", "[99, 98, 97, 96]"),
            log("penalty", "1234")
        ]
    );

    assert_eq!(res.messages, 
        vec![
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mAAPL"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: h("addr0000"), 
                    amount: Uint128(99u128)
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mGOOG"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: h("addr0000"), 
                    amount: Uint128(98u128)   
                })
                .unwrap(),
                send: vec![],
            }),CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mMSFT"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: h("addr0000"), 
                    amount: Uint128(97u128)
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mNFLX"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: h("addr0000"), 
                    amount: Uint128(96u128) 
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20HandleMsg::TransferFrom {
                    owner: h("addr0000"),
                    amount: Uint128(13u128),
                    recipient: h("collector"),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20HandleMsg::BurnFrom {
                    owner: h("addr0000"),
                    amount: Uint128(1234u128),
                })
                .unwrap(),
                send: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyQueryMsg::Redeem {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128(100_000_000u128),
                    inventory: vec![
                        Uint128(7_290_053_159u128), Uint128(319_710_128u128),
                        Uint128(14_219_281_228u128), Uint128(224_212_221u128)
                    ],
                    max_tokens: Uint128(20_000_000u128),
                    redeem_asset_amounts: vec![],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string()
                    ],
                    target_weights: vec![
                        Uint128(20u128),
                        Uint128(20u128),
                        Uint128(20u128),
                        Uint128(20u128)
                    ],
                })
                .unwrap(),
                send: vec![],
            }),
        ]
    );
    assert_eq!(7, res.messages.len());
}

#[test]
fn update_target() {
    let (mut deps, _init_res) = mock_init();
    mock_querier_setup(&mut deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    let new_target: Vec<Asset> = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128(10),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128(5),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128(35),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGME"),
            },
            amount: Uint128(50),
        },
    ];
    let msg = HandleMsg::UpdateTarget { target: new_target };

    let env = mock_env(consts::owner(), &[]);
    let res = handle(&mut deps, env, msg).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "reset_target"),
            log("prev_assets", "[mAAPL, mGOOG, mMSFT, mNFLX]"),
            log("prev_targets", "[20, 20, 20, 20]"),
            log("updated_assets", "[mAAPL, mGOOG, mMSFT, mGME, mNFLX]"),
            log("updated_targets", "[10, 5, 35, 50, 0]"),
        ]
    );

    assert_eq!(res.messages, vec![]);
}

#[test]
fn decommission_cluster() {
    let (mut deps, _init_res) = mock_init();
    mock_querier_setup(&mut deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, true);

    let msg = HandleMsg::Decommission {};

    let env = mock_env("owner0001", &[]);
    let res = handle(&mut deps, env, msg.clone()).unwrap_err();

    match res {
        StdError::Unauthorized { .. } => {}
        _ => panic!("DO NOT ENTER HERE"),
    }

    let env = mock_env(consts::factory(), &[]);

    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(res.log, vec![log("action", "decommission_asset")]);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, false);

    assert_eq!(res.messages, vec![]);

    let res = handle(&mut deps, env.clone(), msg).unwrap_err();

    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot decommission an already decommissioned cluster")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let asset_amounts = consts::asset_amounts();
    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let msg = HandleMsg::Mint {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let res = handle(&mut deps, env.clone(), msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot call mint on a decommissioned cluster")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::Burn {
        max_tokens: Uint128(20_000_000),
        asset_amounts: Some(asset_amounts),
    };

    let res = handle(&mut deps, env.clone(), msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot call non pro-rata redeem on a decommissioned cluster")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = HandleMsg::Burn {
        max_tokens: Uint128(20_000_000),
        asset_amounts: None,
    };

    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(res.log, vec![
        log("action", "receive:burn"),
        log("sender", "factory"),
        log("burn_amount", "1234"),
        log("token_cost", "1247"),
        log("kept_as_fee", "13"),
        log("asset_amounts", "[]"),
        log("redeem_totals", "[99, 98, 97, 96]"),
        log("penalty", "1234")
    ]);
}
