#[cfg(test)]
mod tests {

    use crate::contract::{handle, init, query};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{from_binary, log, Decimal, HumanAddr, StdError, Uint128};

    use nebula_protocol::staking::{
        ConfigResponse, HandleMsg, InitMsg, PoolInfoResponse, QueryMsg,
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
        };

        let env = mock_env("addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = init(&mut deps, env, msg).unwrap();

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
                owner: HumanAddr::from("owner"),
                nebula_token: HumanAddr::from("reward"),
            },
            config
        );
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
        };

        let env = mock_env("addr", &[]);
        let _res = init(&mut deps, env.clone(), msg).unwrap();

        // update owner
        let env = mock_env("owner", &[]);
        let msg = HandleMsg::UpdateConfig {
            owner: Some(HumanAddr("owner2".to_string())),
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
                owner: HumanAddr::from("owner2"),
                nebula_token: HumanAddr::from("reward"),
            },
            config
        );

        // unauthorized err
        let env = mock_env("owner", &[]);
        let msg = HandleMsg::UpdateConfig { owner: None };

        let res = handle(&mut deps, env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_register() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
        };

        let env = mock_env("addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = init(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::RegisterAsset {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("staking"),
        };

        // failed with unauthorized error
        let env = mock_env("addr", &[]);
        let res = handle(&mut deps, env, msg.clone()).unwrap_err();
        match res {
            StdError::Unauthorized { .. } => {}
            _ => panic!("DO NOT ENTER HERE"),
        }

        let env = mock_env("owner", &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(
            res.log,
            vec![log("action", "register_asset"), log("asset_token", "asset"),]
        );

        let res = query(
            &deps,
            QueryMsg::PoolInfo {
                asset_token: HumanAddr::from("asset"),
            },
        )
        .unwrap();
        let pool_info: PoolInfoResponse = from_binary(&res).unwrap();
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
    }
}
