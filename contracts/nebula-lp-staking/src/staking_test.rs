#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::mock_querier::mock_dependencies_with_querier;
    use cosmwasm_std::testing::{mock_dependencies, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{
        attr, from_binary, to_binary, Coin, CosmosMsg, Decimal, StdError, Uint128, WasmMsg,
    };
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
    use nebula_protocol::staking::{
        Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
        RewardInfoResponseItem,
    };

    use terraswap::asset::{Asset, AssetInfo};
    use terraswap::pair::ExecuteMsg as PairExecuteMsg;

    #[test]
    fn test_bond_tokens() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("nebtoken"),
            terraswap_factory: ("terraswap-factory"),
        };

        let env = mock_info("addr", &[]);
        let _res = instantiate(deps.as_mut(), env, msg).unwrap();

        let msg = ExecuteMsg::RegisterAsset {
            asset_token: ("asset"),
            staking_token: ("staking"),
        };

        let env = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env, msg.clone()).unwrap();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("addr"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: ("asset"),
                })
                .unwrap(),
            ),
        });

        let env = mock_info("staking", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();
        let data = query(
            deps.as_ref(),
            QueryMsg::RewardInfo {
                asset_token: Some(("asset")),
                staker_addr: ("addr"),
            },
        )
        .unwrap();
        let res: RewardInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            res,
            RewardInfoResponse {
                staker_addr: ("addr"),
                reward_infos: vec![RewardInfoResponseItem {
                    asset_token: ("asset"),
                    pending_reward: Uint128::zero(),
                    bond_amount: Uint128::new(100u128),
                }],
            }
        );

        let data = query(
            deps.as_ref(),
            QueryMsg::PoolInfo {
                asset_token: ("asset"),
            },
        )
        .unwrap();

        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: ("asset"),
                staking_token: ("staking"),
                total_bond_amount: Uint128::new(100u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        // bond 100 more tokens from other account
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("addr2"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: ("asset"),
                })
                .unwrap(),
            ),
        });
        let env = mock_info("staking", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        let data = query(
            deps.as_ref(),
            QueryMsg::PoolInfo {
                asset_token: ("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: ("asset"),
                staking_token: ("staking"),
                total_bond_amount: Uint128::new(200u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        // failed with unauthorized
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("addr"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: ("asset"),
                })
                .unwrap(),
            ),
        });

        let env = mock_info("staking2", &[]);
        let res = execute(deps.as_mut(), env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_unbond() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("nebtoken"),
            terraswap_factory: ("terraswap-factory"),
        };

        let env = mock_info("addr", &[]);
        let _res = instantiate(deps.as_mut(), env, msg).unwrap();

        // register asset
        let msg = ExecuteMsg::RegisterAsset {
            asset_token: ("asset"),
            staking_token: ("staking"),
        };

        let env = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env, msg.clone()).unwrap();

        // bond 100 tokens
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("addr"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: ("asset"),
                })
                .unwrap(),
            ),
        });
        let env = mock_info("staking", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        // unbond 150 tokens; failed
        let msg = ExecuteMsg::Unbond {
            asset_token: ("asset"),
            amount: Uint128::new(150u128),
        };

        let env = mock_info("addr", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap_err();
        match res {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "Cannot unbond more than bond amount");
            }
            _ => panic!("Must return generic error"),
        };

        // normal unbond
        let msg = ExecuteMsg::Unbond {
            asset_token: ("asset"),
            amount: Uint128::new(100u128),
        };

        let env = mock_info("addr", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap();
        assert_eq!(
            res.messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ("staking"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: ("addr"),
                    amount: Uint128::new(100u128),
                })
                .unwrap(),
                funds: vec![],
            })]
        );

        let data = query(
            deps.as_ref(),
            QueryMsg::PoolInfo {
                asset_token: ("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: ("asset"),
                staking_token: ("staking"),
                total_bond_amount: Uint128::zero(),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );

        let data = query(
            deps.as_ref(),
            QueryMsg::RewardInfo {
                asset_token: None,
                staker_addr: ("addr"),
            },
        )
        .unwrap();
        let res: RewardInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            res,
            RewardInfoResponse {
                staker_addr: ("addr"),
                reward_infos: vec![],
            }
        );
    }

    #[test]
    fn test_auto_stake() {
        let mut deps = mock_dependencies_with_querier(20, &[]);
        deps.querier.with_pair_info(("pair"));
        deps.querier.with_pool_assets([
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
                amount: Uint128::from(100u128),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: ("asset"),
                },
                amount: Uint128::from(1u128),
            },
        ]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("nebtoken"),
            terraswap_factory: ("terraswap_factory"),
        };

        let env = mock_info("addr", &[]);
        let _res = instantiate(deps.as_mut(), env, msg).unwrap();

        let msg = ExecuteMsg::RegisterAsset {
            asset_token: ("asset"),
            staking_token: ("lptoken"),
        };

        let env = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env, msg.clone()).unwrap();

        // no token asset
        let msg = ExecuteMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::new(100u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::new(100u128),
                },
            ],
            slippage_tolerance: None,
        };
        let env = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(100u128),
            }],
        );
        let res = execute(deps.as_mut(), env, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("Missing token asset"));

        // no native asset
        let msg = ExecuteMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: ("asset"),
                    },
                    amount: Uint128::from(1u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: ("asset"),
                    },
                    amount: Uint128::from(1u128),
                },
            ],
            slippage_tolerance: None,
        };
        let env = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap_err();
        assert_eq!(res, StdError::generic_err("Missing native asset"));

        let msg = ExecuteMsg::AutoStake {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::new(100u128),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: ("asset"),
                    },
                    amount: Uint128::new(1u128),
                },
            ],
            slippage_tolerance: None,
        };

        // attempt with no coins
        let env = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env, msg.clone()).unwrap_err();
        assert_eq!(
            res,
            StdError::generic_err(
                "Native token balance missmatch between the argument and the transferred"
            )
        );

        let env = mock_info(
            "addr0000",
            &[Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(100u128),
            }],
        );
        let res = execute(deps.as_mut(), env, msg.clone()).unwrap();
        assert_eq!(
            res.messages,
            vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ("asset"),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: ("addr0000"),
                        recipient: (MOCK_CONTRACT_ADDR),
                        amount: Uint128::new(1u128),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ("asset"),
                    msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                        spender: ("pair"),
                        amount: Uint128::new(1),
                        expires: None,
                    })
                    .unwrap(),
                    funds: vec![],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: ("pair"),
                    msg: to_binary(&PairExecuteMsg::ProvideLiquidity {
                        assets: [
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string()
                                },
                                amount: Uint128::new(99u128),
                            },
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: ("asset")
                                },
                                amount: Uint128::new(1u128),
                            },
                        ],
                        slippage_tolerance: None,
                    })
                    .unwrap(),
                    funds: vec![Coin {
                        denom: "uusd".to_string(),
                        amount: Uint128::new(99u128), // 1% tax
                    }],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: (MOCK_CONTRACT_ADDR),
                    msg: to_binary(&ExecuteMsg::AutoStakeHook {
                        asset_token: ("asset"),
                        staking_token: ("lptoken"),
                        staker_addr: ("addr0000"),
                        prev_staking_token_amount: Uint128::zero(),
                    })
                    .unwrap(),
                    funds: vec![],
                })
            ]
        );

        deps.querier.with_token_balance(Uint128::new(100u128)); // recive 100 lptoken

        // wrong asset
        let msg = ExecuteMsg::AutoStakeHook {
            asset_token: ("asset1"),
            staking_token: ("lptoken"),
            staker_addr: ("addr0000"),
            prev_staking_token_amount: Uint128::zero(),
        };
        let env = mock_info(MOCK_CONTRACT_ADDR, &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap_err(); // pool not found error

        // valid msg
        let msg = ExecuteMsg::AutoStakeHook {
            asset_token: ("asset"),
            staking_token: ("lptoken"),
            staker_addr: ("addr0000"),
            prev_staking_token_amount: Uint128::zero(),
        };

        // unauthorized attempt
        let env = mock_info("addr0000", &[]);
        let res = execute(deps.as_mut(), env, msg.clone()).unwrap_err();
        assert_eq!(res, StdError::generic_err("unauthorized"));

        // successfull attempt
        let env = mock_info(MOCK_CONTRACT_ADDR, &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "bond"),
                attr("staker_addr", "addr0000"),
                attr("asset_token", "asset"),
                attr("amount", "100"),
            ]
        );

        let data = query(
            deps.as_ref(),
            QueryMsg::PoolInfo {
                asset_token: ("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&data).unwrap();
        assert_eq!(
            pool_info,
            PoolInfoResponse {
                asset_token: ("asset"),
                staking_token: ("lptoken"),
                total_bond_amount: Uint128::new(100u128),
                reward_index: Decimal::zero(),
                pending_reward: Uint128::zero(),
            }
        );
    }
}
