#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::state::{rewards_read, RewardInfo};
    use cosmwasm_std::testing::{mock_dependencies, mock_info};
    use cosmwasm_std::{from_binary, to_binary, CosmosMsg, Decimal, Uint128, WasmMsg};
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
    use nebula_protocol::staking::{
        Cw20HookMsg, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
        RewardInfoResponseItem,
    };

    #[test]
    fn test_deposit_reward() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("reward"),
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

        // factory deposit 100 reward tokens
        // premium is 0, so rewards distributed 80:20
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![(("asset"), Uint128::new(100u128))],
                })
                .unwrap(),
            ),
        });
        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

        // Check pool state
        let res: PoolInfoResponse = from_binary(
            &query(
                deps.as_ref(),
                QueryMsg::PoolInfo {
                    asset_token: ("asset"),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            res.clone(),
            PoolInfoResponse {
                total_bond_amount: Uint128::new(100u128),
                reward_index: Decimal::one(),
                ..res
            }
        );
    }

    #[test]
    fn test_deposit_reward_when_no_bonding() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("reward"),
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

        // factory deposit 100 reward tokens
        // premium is 0, so rewards distributed 80:20
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![(("asset"), Uint128::new(100u128))],
                })
                .unwrap(),
            ),
        });
        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env.clone(), msg.clone()).unwrap();

        // Check pool state
        let res: PoolInfoResponse = from_binary(
            &query(
                deps.as_ref(),
                QueryMsg::PoolInfo {
                    asset_token: ("asset"),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            res.clone(),
            PoolInfoResponse {
                reward_index: Decimal::zero(),
                pending_reward: Uint128::new(100u128),
                ..res
            }
        );
    }

    #[test]
    fn test_before_share_changes() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("reward"),
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

        // factory deposit 100 reward tokens
        // premium is 0, so rewards distributed 80:20
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![(("asset"), Uint128::new(100u128))],
                })
                .unwrap(),
            ),
        });

        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        let user_addr = ("addr");
        let asset_addr = ("asset");

        let reward_bucket = rewards_read(deps.storage, &user_addr);
        let reward_info: RewardInfo = reward_bucket.load(asset_addr.as_str().as_bytes()).unwrap();
        assert_eq!(
            RewardInfo {
                pending_reward: Uint128::zero(),
                bond_amount: Uint128::new(100u128),
                index: Decimal::zero(),
            },
            reward_info
        );

        // bond 100 more tokens
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

        let reward_bucket = rewards_read(deps.storage, &user_addr);
        let reward_info: RewardInfo = reward_bucket.load(asset_addr.as_str().as_bytes()).unwrap();
        assert_eq!(
            RewardInfo {
                pending_reward: Uint128::new(100u128),
                bond_amount: Uint128::new(200u128),
                index: Decimal::one(),
            },
            reward_info
        );

        // factory deposit 100 reward tokens; = 1.0 + 0.5 = 1.5 is reward_index
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![(("asset"), Uint128::new(100u128))],
                })
                .unwrap(),
            ),
        });
        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        // unbond
        let msg = ExecuteMsg::Unbond {
            asset_token: ("asset"),
            amount: Uint128::new(100u128),
        };
        let env = mock_info("addr", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        let reward_bucket = rewards_read(deps.storage, &user_addr);
        let reward_info: RewardInfo = reward_bucket.load(asset_addr.as_str().as_bytes()).unwrap();
        assert_eq!(
            RewardInfo {
                pending_reward: Uint128::new(200u128),
                bond_amount: Uint128::new(100u128),
                index: Decimal::from_ratio(150u128, 100u128),
            },
            reward_info
        );
    }

    #[test]
    fn test_withdraw() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("reward"),
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

        // factory deposit 100 reward tokens
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(100u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![(("asset"), Uint128::new(100u128))],
                })
                .unwrap(),
            ),
        });
        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        let msg = ExecuteMsg::Withdraw {
            asset_token: Some(("asset")),
        };
        let env = mock_info("addr", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ("reward"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: ("addr"),
                    amount: Uint128::new(100u128),
                })
                .unwrap(),
                funds: vec![],
            })]
        );
    }

    #[test]
    fn withdraw_multiple_rewards() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: ("owner"),
            nebula_token: ("reward"),
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

        let msg = ExecuteMsg::RegisterAsset {
            asset_token: ("asset2"),
            staking_token: ("staking2"),
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

        // bond second 1000 tokens
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("addr"),
            amount: Uint128::new(1000u128),
            msg: Some(
                to_binary(&Cw20HookMsg::Bond {
                    asset_token: ("asset2"),
                })
                .unwrap(),
            ),
        });
        let env = mock_info("staking2", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

        // factory deposit asset
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ("factory"),
            amount: Uint128::new(300u128),
            msg: Some(
                to_binary(&Cw20HookMsg::DepositReward {
                    rewards: vec![
                        (("asset"), Uint128::new(100u128)),
                        (("asset2"), Uint128::new(200u128)),
                    ],
                })
                .unwrap(),
            ),
        });
        let env = mock_info("reward", &[]);
        let _res = execute(deps.as_mut(), env, msg).unwrap();

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
                reward_infos: vec![
                    RewardInfoResponseItem {
                        asset_token: ("asset"),
                        bond_amount: Uint128::new(100u128),
                        pending_reward: Uint128::new(100u128),
                    },
                    RewardInfoResponseItem {
                        asset_token: ("asset2"),
                        bond_amount: Uint128::new(1000u128),
                        pending_reward: Uint128::new(200u128),
                    },
                ],
            }
        );

        // withdraw all
        let msg = ExecuteMsg::Withdraw { asset_token: None };
        let env = mock_info("addr", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ("reward"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: ("addr"),
                    amount: Uint128::new(300u128),
                })
                .unwrap(),
                funds: vec![],
            })]
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
                reward_infos: vec![
                    RewardInfoResponseItem {
                        asset_token: ("asset"),
                        bond_amount: Uint128::new(100u128),
                        pending_reward: Uint128::zero(),
                    },
                    RewardInfoResponseItem {
                        asset_token: ("asset2"),
                        bond_amount: Uint128::new(1000u128),
                        pending_reward: Uint128::zero(),
                    },
                ],
            }
        );
    }
}
