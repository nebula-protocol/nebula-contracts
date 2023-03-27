use crate::contract::*;
use crate::error::ContractError;
use crate::state::*;
use crate::testing::mock_querier::{
    consts, mock_dependencies, mock_init, mock_querier_setup, token_data,
};
use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::*;
use cw2::{get_contract_version, ContractVersion};
use cw20::Cw20ExecuteMsg;
use nebula_protocol::cluster::{
    ClusterConfig, ClusterInfoResponse, ConfigResponse, InstantiateMsg, MigrateMsg,
};
use nebula_protocol::cluster::{
    ClusterStateResponse, ExecuteMsg, QueryMsg as ClusterQueryMsg, TargetResponse,
};
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
use pretty_assertions::assert_eq;
use std::str::FromStr;

/// Convenience function for creating inline String
pub fn h(s: &str) -> String {
    s.to_string()
}

#[macro_export]
macro_rules! q {
    ($deps:expr, $val_type:ty, $env:expr, $msg: expr) => {{
        let res = query($deps, $env, $msg).unwrap();
        let val: $val_type = from_binary(&res).unwrap();
        val
    }};
}

#[test]
fn proper_initialization() {
    let (deps, init_res) = mock_init();
    assert_eq!(0, init_res.messages.len());

    // make sure target was saved
    let value = q!(
        deps.as_ref(),
        TargetResponse,
        mock_env(),
        ClusterQueryMsg::Target {}
    );
    assert_eq!(
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mAAPL"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mGOOG"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mMSFT"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mNFLX"),
                },
                amount: Uint128::new(15)
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5)
            },
        ],
        value.target
    );

    let config = q!(
        deps.as_ref(),
        ConfigResponse,
        mock_env(),
        ClusterQueryMsg::Config {}
    );
    assert_eq!(
        ClusterConfig {
            name: String::from("test_cluster"),
            description: String::from("description"),
            owner: Addr::unchecked("owner"),
            cluster_token: Some(Addr::unchecked("cluster")),
            pricing_oracle: Addr::unchecked("pricing_oracle"),
            target_oracle: Addr::unchecked("target_oracle"),
            penalty: Addr::unchecked("penalty"),
            factory: Addr::unchecked("factory"),
            active: true,
        },
        config.config,
    );

    let info = q!(
        deps.as_ref(),
        ClusterInfoResponse,
        mock_env(),
        ClusterQueryMsg::ClusterInfo {}
    );
    assert_eq!(
        ClusterInfoResponse {
            name: "test_cluster".to_string(),
            description: "description".to_string()
        },
        info,
    )
}

#[test]
fn bad_initialization() {
    let mut deps = mock_dependencies(&[]);
    deps = mock_querier_setup(deps);

    // duplicate target assets
    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uasset".to_string(),
                },
                amount: Uint128::new(60),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uasset".to_string(),
                },
                amount: Uint128::new(40),
            },
        ],
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };
    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidAssets {});

    // initial zero weight
    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(0),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uasset".to_string(),
                },
                amount: Uint128::new(100),
            },
        ],
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };
    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("Initial weights cannot contain zero".to_string(),)
    );
}

#[test]
fn update_config() {
    let (mut deps, init_res) = mock_init();
    assert_eq!(0, init_res.messages.len());

    // unauthorized update
    let info = mock_info("sender0001", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        name: Some("cluster0001".to_string()),
        description: Some("description".to_string()),
        cluster_token: Some("token0001".to_string()),
        pricing_oracle: Some("oracle0001".to_string()),
        target_oracle: Some("owner".to_string()),
        penalty: Some("penalty0001".to_string()),
        target: Some(vec![
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
        ]),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // successful update
    let info = mock_info("owner", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let config = read_config(&deps.storage).unwrap();
    assert_eq!(
        config,
        ClusterConfig {
            name: "cluster0001".to_string(),
            description: "description".to_string(),
            owner: Addr::unchecked("owner0001"),
            cluster_token: Some(Addr::unchecked("token0001")),
            factory: Addr::unchecked("factory"),
            pricing_oracle: Addr::unchecked("oracle0001"),
            target_oracle: Addr::unchecked("owner"),
            penalty: Addr::unchecked("penalty0001"),
            active: true
        }
    )
}

#[test]
fn initialize_cluster() {
    let mut deps = mock_dependencies(&[]);
    deps.querier
        .set_token(
            consts::cluster_token(),
            token_data::<Vec<(&str, u128)>, &str>("Cluster Token", "CLUSTER", 6, 0, vec![]),
        )
        .set_native_balance("uasset", MOCK_CONTRACT_ADDR, 1_000_000u128)
        .set_native_balance("uluna", MOCK_CONTRACT_ADDR, 1_000_000u128)
        .set_oracle_prices(vec![
            ("uasset", Decimal::one()),
            ("uluna", Decimal::from_str("62.5").unwrap()),
        ]);

    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uasset".to_string(),
                },
                amount: Uint128::new(50),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(50),
            },
        ],
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };
    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg);

    // cannot initialize with incomplete weight supplied
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: vec![Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::new(1_000_000u128),
        }],
        min_tokens: Some(Uint128::new(1_000_000)),
    };
    let info = mock_info("addr0000", &[coin(1_000_000u128, "uluna")]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic(
            "Initial cluster assets must be a nonzero multiple of target weights at index 0"
                .to_string()
        )
    );

    // cannot initialize with weight invariant supplied
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uasset".to_string(),
                },
                amount: Uint128::new(1_500_000u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(1_000_000u128),
            },
        ],
        min_tokens: Some(Uint128::new(1_000_000)),
    };
    let info = mock_info(
        "addr0000",
        &[coin(1_500_000u128, "uasset"), coin(1_000_000u128, "uluna")],
    );
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic(
            "Initial cluster assets have weight invariant at index 1".to_string()
        )
    );

    let asset_amounts = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uasset".to_string(),
            },
            amount: Uint128::new(1_000_000u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::new(1_000_000u128),
        },
    ];

    // does not provide min_tokens
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };
    let info = mock_info(
        "addr0000",
        &[coin(1_000_000u128, "uasset"), coin(1_000_000u128, "uluna")],
    );
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::Generic("Cluster is uninitialized. To initialize it with your mint cluster, provide min_tokens as the amount of cluster tokens you want to start with.".to_string()));

    // successful initialization
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts,
        min_tokens: Some(Uint128::new(1_000_000)),
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cluster".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: "addr0000".to_string(),
                amount: Uint128::new(1_000_000)
            })
            .unwrap(),
            funds: vec![]
        }))],
    )
}

#[test]
fn mint() {
    let (mut deps, _) = mock_init();
    deps = mock_querier_setup(deps);
    // Asset :: BASE_DENOM Price :: Balance (µ)     (+ proposed   ) :: %
    // ---
    // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
    // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
    // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
    // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389

    deps.querier.set_oracle_prices(vec![
        ("mAAPL", Decimal::from_str("135.18").unwrap()),
        ("mGOOG", Decimal::from_str("1780.03").unwrap()),
        ("mMSFT", Decimal::from_str("222.42").unwrap()),
        ("mNFLX", Decimal::from_str("540.82").unwrap()),
    ]);

    let asset_amounts = consts::asset_amounts();
    let addr = "addr0000";

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    // invalid assets create
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("invalidtoken0001"),
            },
            amount: Uint128::new(1_000_000u128),
        }],
        min_tokens: None,
    };
    let info = mock_info(addr, &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidAssets {});

    // NOTE: this test is not working right now because the execution would persist the state and not revert it (just throw the error).
    // cluster tokens to be minted is below min_tokens specified
    // let msg = ExecuteMsg::RebalanceCreate {
    //     asset_amounts: vec![Asset {
    //         info: AssetInfo::Token {
    //             contract_addr: Addr::unchecked("mAAPL"),
    //         },
    //         amount: Uint128::new(42_000_000),
    //     }],
    //     min_tokens: Some(Uint128::new(99)),
    // };
    // let info = mock_info(addr, &[coin(42_000_000, "uluna")]);
    // let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    // assert_eq!(
    //     res,
    //     ContractError::BelowMinTokens(Uint128::new(98), Uint128::new(99))
    // );

    // correct create msg
    let mint_msg = ExecuteMsg::RebalanceCreate {
        asset_amounts,
        min_tokens: None,
    };

    // unsupported assets sent along with the tx
    let info = mock_info(addr, &[coin(1_000_000u128, "uasset")]);
    let res = execute(deps.as_mut(), mock_env(), info, mint_msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("Unsupported assets were sent to the create function".to_string())
    );

    // correct msg but the prices are stale
    let info = mock_info(addr, &[coin(42_000_000, "uluna")]);
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1_571_797_480u64);
    let res = execute(deps.as_mut(), env, info.clone(), mint_msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::Std(StdError::generic_err("oracle prices are stale"))
    );

    // successful create
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info, mint_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "mint"),
            attr("sender", "addr0000"),
            attr("mint_to_sender", "98"),
            attr("penalty", "1234"),
            attr("fee_amt", "1"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mAAPL"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(125_000_000),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mGOOG"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::zero(),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mMSFT"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(149_000_000),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mNFLX"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(50_090_272),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyExecuteMsg::PenaltyCreate {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128::new(1_000_000_000),
                    inventory: vec![
                        Uint128::new(0u128),
                        Uint128::new(0u128),
                        Uint128::new(0u128),
                        Uint128::new(0u128),
                        Uint128::new(0u128),
                    ],
                    create_asset_amounts: vec![
                        Uint128::new(125_000_000),
                        Uint128::zero(),
                        Uint128::new(149_000_000),
                        Uint128::new(50_090_272),
                        Uint128::new(42_000_000),
                    ],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string(),
                        "62.5".to_string(),
                    ],
                    target_weights: vec![
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(15u128),
                        Uint128::new(5u128),
                    ],
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: Uint128::new(1u128),
                    recipient: h("collector"),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: Uint128::new(98),
                    recipient: "addr0000".to_string(),
                })
                .unwrap(),
                funds: vec![],
            }))
        ]
    );

    // check inventory post-mint
    let res = query(deps.as_ref(), mock_env(), ClusterQueryMsg::ClusterState {}).unwrap();
    let response: ClusterStateResponse = from_binary(&res).unwrap();
    assert_eq!(
        ClusterStateResponse {
            outstanding_balance_tokens: Uint128::from(1_000_000_000u128),
            prices: vec![
                "135.18".to_string(),
                "1780.03".to_string(),
                "222.42".to_string(),
                "540.82".to_string(),
                "62.5".to_string(),
            ],
            inv: vec![
                Uint128::new(125_000_000u128),
                Uint128::zero(),
                Uint128::new(149_000_000),
                Uint128::new(50_090_272),
                Uint128::new(42_000_000),
            ],
            penalty: "penalty".to_string(),
            cluster_token: "cluster".to_string(),
            target: vec![
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("mAAPL"),
                    },
                    amount: Uint128::new(20,),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("mGOOG"),
                    },
                    amount: Uint128::new(20,),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("mMSFT"),
                    },
                    amount: Uint128::new(20,),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("mNFLX"),
                    },
                    amount: Uint128::new(15,),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::new(5,),
                },
            ],
            cluster_contract_address: "cosmos2contract".to_string(),
            active: true
        },
        response
    );
}

#[test]
fn burn() {
    let (mut deps, _init_res) = mock_init();
    deps = mock_querier_setup(deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    // mint first to have inventory assets to redeem
    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = ExecuteMsg::RebalanceCreate {
        asset_amounts,
        min_tokens: None,
    };

    let addr = "addr0000";
    let info = mock_info(addr, &[coin(42_000_000u128, "uluna")]);
    let env = mock_env();
    let _res = execute(deps.as_mut(), env, info, mint_msg).unwrap();

    let asset_amounts = Some(vec![
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
            amount: Uint128::new(0),
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
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::new(20),
        },
    ]);
    let info = mock_info("addr0000", &[]);
    let env = mock_env();

    // cannot redeem because input max_tokens is too low
    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(1_000),
        asset_amounts: asset_amounts.clone(),
    };
    let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::AboveMaxTokens(Uint128::new(1247), Uint128::new(1000))
    );

    // successful redeem
    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts,
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "receive:burn"),
            attr("sender", "addr0000"),
            attr("burn_amount", "1234"),
            attr("token_cost", "1247"),
            attr("kept_as_fee", "13"),
            attr("asset_amounts", "[20, 0, 20, 20, 20]"),
            attr("redeem_totals", "[99, 0, 97, 96, 95]"),
            attr("penalty", "1234")
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mAAPL"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(99u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mMSFT"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(97u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mNFLX"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(96u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "addr0000".to_string(),
                amount: coins(95u128, "uluna")
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    amount: Uint128::new(13u128),
                    recipient: h("collector"),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
                    owner: "addr0000".to_string(),
                    amount: Uint128::new(1234u128),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyExecuteMsg::PenaltyRedeem {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128::new(100_000_000u128),
                    inventory: vec![
                        Uint128::new(125_000_000u128),
                        Uint128::zero(),
                        Uint128::new(149_000_000),
                        Uint128::new(50_090_272),
                        Uint128::new(42_000_000),
                    ],
                    max_tokens: Uint128::new(20_000_000u128),
                    redeem_asset_amounts: vec![
                        Uint128::new(20),
                        Uint128::zero(),
                        Uint128::new(20),
                        Uint128::new(20),
                        Uint128::new(20),
                    ],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string(),
                        "62.5".to_string(),
                    ],
                    target_weights: vec![
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(15u128),
                        Uint128::new(5u128),
                    ],
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn update_target() {
    let new_target: Vec<Asset> = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mAAPL"),
            },
            amount: Uint128::new(10),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mGOOG"),
            },
            amount: Uint128::new(5),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mMSFT"),
            },
            amount: Uint128::new(35),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mGME"),
            },
            amount: Uint128::new(35),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mGE"),
            },
            amount: Uint128::new(5),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Uint128::new(10),
        },
    ];

    // cluster token not set
    let mut deps = mock_dependencies(&[]);
    deps = mock_querier_setup(deps);
    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: None,
        target: consts::target_assets_stage(),
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };
    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let msg = ExecuteMsg::UpdateTarget {
        target: new_target.clone(),
    };
    let info = mock_info(consts::owner().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::ClusterTokenNotSet {});

    let msg = ExecuteMsg::Decommission {};
    let info = mock_info(consts::factory().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::ClusterTokenNotSet {});

    let (mut deps, _init_res) = mock_init();

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    // mint first
    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = ExecuteMsg::RebalanceCreate {
        asset_amounts,
        min_tokens: None,
    };

    let addr = "addr0000";
    let info = mock_info(addr, &[coin(42_000_000u128, "uluna")]);
    let env = mock_env();
    let _res = execute(deps.as_mut(), env, info, mint_msg).unwrap();

    // invalid assets update
    let msg = ExecuteMsg::UpdateTarget {
        target: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("token0001"),
            },
            amount: Uint128::new(20),
        }],
    };
    let info = mock_info(consts::owner().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::InvalidAssets {});

    let msg = ExecuteMsg::UpdateTarget { target: new_target };

    // unauthorized update
    let info = mock_info("imposter0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // successful update
    let info = mock_info(consts::owner().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "reset_target"),
            attr("prev_assets", "[mAAPL, mGOOG, mMSFT, mNFLX, uluna]"),
            attr("prev_targets", "[20, 20, 20, 15, 5]"),
            attr(
                "updated_assets",
                "[mAAPL, mGOOG, mMSFT, mGME, mGE, uluna, mNFLX]"
            ),
            attr("updated_targets", "[10, 5, 35, 35, 5, 10, 0]"),
        ]
    );

    assert_eq!(res.messages, vec![]);

    // cannot call create with zero-weight target
    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: vec![Asset {
            info: AssetInfo::Token {
                contract_addr: Addr::unchecked("mNFLX"),
            },
            amount: Uint128::new(1_000_000u128),
        }],
        min_tokens: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic(
            "Cannot call create with non-zero asset amount when target weight is zero for asset mNFLX".to_string(),
        )
    );
}

#[test]
fn decommission_cluster() {
    let (mut deps, _init_res) = mock_init();
    deps = mock_querier_setup(deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, true);

    // mint first to have inventory assets to redeem
    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = ExecuteMsg::RebalanceCreate {
        asset_amounts,
        min_tokens: None,
    };

    let addr = "addr0000";
    let info = mock_info(addr, &[coin(42_000_000u128, "uluna")]);
    let env = mock_env();
    let _res = execute(deps.as_mut(), env, info, mint_msg).unwrap();

    let msg = ExecuteMsg::Decommission {};

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();

    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info(consts::factory().as_str(), &[]);

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(res.attributes, vec![attr("action", "decommission_asset")]);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, false);

    assert_eq!(res.messages, vec![]);

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::ClusterAlreadyDecommissioned {});

    let asset_amounts = consts::asset_amounts();
    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(res, ContractError::ClusterAlreadyDecommissioned {});

    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts: Some(asset_amounts),
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic(
            "Cannot call non pro-rata redeem on a decommissioned cluster".to_string()
        )
    );

    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "receive:burn"),
            attr("sender", "factory"),
            attr("burn_amount", "1234"),
            attr("token_cost", "1247"),
            attr("kept_as_fee", "13"),
            attr("asset_amounts", "[]"),
            attr("redeem_totals", "[99, 0, 97, 96, 95]"),
            attr("penalty", "1234")
        ]
    );

    // cannot update decommissioned cluster
    let new_target: Vec<Asset> = vec![Asset {
        info: AssetInfo::Token {
            contract_addr: Addr::unchecked("mAAPL"),
        },
        amount: Uint128::new(10),
    }];
    let msg = ExecuteMsg::UpdateTarget { target: new_target };
    let info = mock_info(consts::owner().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::ClusterAlreadyDecommissioned {});
}

#[test]
fn migration() {
    let (mut deps, _init_res) = mock_init();

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-cluster".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
