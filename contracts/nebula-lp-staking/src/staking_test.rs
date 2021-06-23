#[cfg(test)]
mod tests {
    use crate::contract::{handle, init, query};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{
        from_binary, to_binary, CosmosMsg, Decimal, HumanAddr, StdError, Uint128, WasmMsg,
    };
    use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
    use nebula_protocol::staking::{
        Cw20HookMsg, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg, RewardInfoResponse,
        RewardInfoResponseItem,
    };

    #[test]
    fn test_bond_tokens() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
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
            nebula_token: HumanAddr::from("reward"),
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
}