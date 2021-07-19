#[cfg(test)]
mod tests {
    use crate::contract::{handle, init, query};
    use crate::mock_querier::mock_dependencies_with_querier;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        from_binary, log, to_binary, Coin, CosmosMsg, Decimal, HumanAddr, StdError, Uint128, WasmMsg,
    };
    use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
    use nebula_protocol::staking::{
        Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
        RewardInfoResponseItem,
    };

    use terraswap::asset::{Asset, AssetInfo};
    use terraswap::pair::HandleMsg as PairHandleMsg;

    #[test]
    fn test_bond_tokens() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("nebtoken"),
            terraswap_factory: HumanAddr::from("terraswap-factory")
        };

        let env = mock_env("addr", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::RegisterAsset {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("staking"),
        };

        let env = mock_env("owner", &[]);
        let _res = handle(&mut deps, env, msg.clone()).unwrap();

        let msg = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("addr"),
            amount: Uint128(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: HumanAddr::from("asset"),
                })
                .unwrap(),
            ),
        });

        let env = mock_env("staking", &[]);
        let _res = handle(&mut deps, env, msg).unwrap();
        let data = query(
            &deps,
            QueryMsg::RewardInfo {
                asset_token: Some(HumanAddr::from("asset")),
                staker_addr: HumanAddr::from("addr"),
            },
        )
        .unwrap();
        let res: RewardInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            res,
            RewardInfoResponse {
                staker_addr: HumanAddr::from("addr"),
                reward_infos: vec![RewardInfoResponseItem {
                    asset_token: HumanAddr::from("asset"),
                    pending_reward: Uint128::zero(),
                    bond_amount: Uint128(100u128),
                }],
            }
        );

        let data = query(
            &deps,
            QueryMsg::PoolInfo {
                asset_token: HumanAddr::from("asset"),
            },
        )
        .unwrap();

        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: HumanAddr::from("asset"),
                staking_token: HumanAddr::from("staking"),
                total_bond_amount: Uint128(100u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        // bond 100 more tokens from other account
        let msg = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("addr2"),
            amount: Uint128(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: HumanAddr::from("asset"),
                })
                .unwrap(),
            ),
        });
        let env = mock_env("staking", &[]);
        let _res = handle(&mut deps, env, msg).unwrap();

        let data = query(
            &deps,
            QueryMsg::PoolInfo {
                asset_token: HumanAddr::from("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: HumanAddr::from("asset"),
                staking_token: HumanAddr::from("staking"),
                total_bond_amount: Uint128(200u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        // failed with unauthorized
        let msg = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("addr"),
            amount: Uint128(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: HumanAddr::from("asset"),
                })
                .unwrap(),
            ),
        });

        let env = mock_env("staking2", &[]);
        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_unbond() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("nebtoken"),
            terraswap_factory: HumanAddr::from("terraswap-factory")
        };

        let env = mock_env("addr", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        // register asset
        let msg = HandleMsg::RegisterAsset {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("staking"),
        };

        let env = mock_env("owner", &[]);
        let _res = handle(&mut deps, env, msg.clone()).unwrap();

        // bond 100 tokens
        let msg = HandleMsg::Receive(Cw20ReceiveMsg {
            sender: HumanAddr::from("addr"),
            amount: Uint128(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: HumanAddr::from("asset"),
                })
                .unwrap(),
            ),
        });
        let env = mock_env("staking", &[]);
        let _res = handle(&mut deps, env, msg).unwrap();

        // unbond 150 tokens; failed
        let msg = HandleMsg::Unbond {
            asset_token: HumanAddr::from("asset"),
            amount: Uint128(150u128),
        };

        let env = mock_env("addr", &[]);
        let res = handle(&mut deps, env, msg).unwrap_err();
        match res {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Cannot unbond more than bond amount");
            }
            _ => panic!("Must return generic error"),
        };

        // normal unbond
        let msg = HandleMsg::Unbond {
            asset_token: HumanAddr::from("asset"),
            amount: Uint128(100u128),
        };

        let env = mock_env("addr", &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: HumanAddr::from("staking"),
                msg: to_binary(&Cw20HandleMsg::Transfer {
                    recipient: HumanAddr::from("addr"),
                    amount: Uint128(100u128),
                })
                .unwrap(),
                send: vec![],
            })]
        );

        let data = query(
            &deps,
            QueryMsg::PoolInfo {
                asset_token: HumanAddr::from("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: HumanAddr::from("asset"),
                staking_token: HumanAddr::from("staking"),
                total_bond_amount: Uint128::zero(),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        let data = query(
            &deps,
            QueryMsg::RewardInfo {
                asset_token: None,
                staker_addr: HumanAddr::from("addr"),
            },
        )
        .unwrap();
        let res: RewardInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            res,
            RewardInfoResponse {
                staker_addr: HumanAddr::from("addr"),
                reward_infos: vec![],
            }
        );
    }

    #[test]
    fn test_auto_stake() {
        let mut deps = mock_dependencies_with_querier(20, &[]);
        deps.querier.with_pair_info(HumanAddr::from("pair"));
        deps.querier.with_pool_assets([
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: HumanAddr::from("asset"),
                },
                amount: Uint128::from(1u128),
            },
        ]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("nebtoken"),
            terraswap_factory: HumanAddr::from("terraswap_factory"),
        };

        let env = mock_env("addr", &[]);
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::RegisterAsset {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("lptoken"),
        };

        let env = mock_env("owner", &[]);
        let _res = handle(&mut deps, env, msg.clone()).unwrap();

        // no token asset
        let msg = HandleMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128(100u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128(100u128),
                },
            ],
            slippage_tolerance: None,
        };
        let env = mock_env(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128(100u128),
            }],
        );
        let res = handle(&mut deps, env, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("Missing token asset"));

        // no native asset
        let msg = HandleMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: HumanAddr::from("asset"),
                    },
                    amount: Uint128::from(1u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: HumanAddr::from("asset"),
                    },
                    amount: Uint128::from(1u128),
                },
            ],
            slippage_tolerance: None,
        };
        let env = mock_env("addr0000", &[]);
        let res = handle(&mut deps, env, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("Missing native asset"));

        let msg = HandleMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128(100u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: HumanAddr::from("asset"),
                    },
                    amount: Uint128(1u128),
                },
            ],
            slippage_tolerance: None,
        };

        // attempt with no coins
        let env = mock_env("addr0000", &[]);
        let res = handle(&mut deps, env, msg.clone()).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err(
                "Native token balance missmatch between the argument and the transferred"
            )
        );

        let env = mock_env(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128(100u128),
            }],
        );
        let res = handle(&mut deps, env, msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: HumanAddr::from("asset"),
                    msg: to_binary(&Cw20HandleMsg::TransferFrom {
                        owner: HumanAddr::from("addr0000"),
                        recipient: HumanAddr::from(MOCK_CONTRACT_ADDR),
                        amount: Uint128(1u128),
                    })
                    .unwrap(),
                    send: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: HumanAddr::from("asset"),
                    msg: to_binary(&Cw20HandleMsg::IncreaseAllowance {
                        spender: HumanAddr::from("pair"),
                        amount: Uint128(1),
                        expires: None,
                    })
                    .unwrap(),
                    send: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: HumanAddr::from("pair"),
                    msg: to_binary(&PairHandleMsg::ProvideLiquidity {
                        assets: [
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string()
                                },
                                amount: Uint128(99u128),
                            },
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: HumanAddr::from("asset")
                                },
                                amount: Uint128(1u128),
                            },
                        ],
                        slippage_tolerance: None,
                    })
                    .unwrap(),
                    send: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128(99u128), // 1% tax
                    }],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&HandleMsg::AutoStakeHook {
                        asset_token: HumanAddr::from("asset"),
                        staking_token: HumanAddr::from("lptoken"),
                        staker_addr: HumanAddr::from("addr0000"),
                        prev_staking_token_amount: Uint128(0),
                    })
                    .unwrap(),
                    send: vec![],
                })
            ]
        );

        deps.querier.with_token_balance(Uint128(100u128)); // recive 100 lptoken

        // wrong asset
        let msg = HandleMsg::AutoStakeHook {
            asset_token: HumanAddr::from("asset1"),
            staking_token: HumanAddr::from("lptoken"),
            staker_addr: HumanAddr::from("addr0000"),
            prev_staking_token_amount: Uint128(0),
        };
        let env = mock_env(MOCK_CONTRACT_ADDR, &[]);
        let _res = handle(&mut deps, env, msg).unwrap_err(); // pool not found error

        // valid msg
        let msg = HandleMsg::AutoStakeHook {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("lptoken"),
            staker_addr: HumanAddr::from("addr0000"),
            prev_staking_token_amount: Uint128(0),
        };

        // unauthorized attempt
        let env = mock_env("addr0000", &[]);
        let res = handle(&mut deps, env, msg.clone()).unwrap_err();
        assert_eq!(res, StdError::unauthorized());

        // successfull attempt
        let env = mock_env(MOCK_CONTRACT_ADDR, &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(
            res.log,
            vec![
                log("action", "bond"),
                log("staker_addr", "addr0000"),
                log("asset_token", "asset"),
                log("amount", "100"),
            ]
        );

        let data = query(
            &deps,
            QueryMsg::PoolInfo {
                asset_token: HumanAddr::from("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: HumanAddr::from("asset"),
                staking_token: HumanAddr::from("lptoken"),
                total_bond_amount: Uint128(100u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );
    }
}
