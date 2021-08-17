#[cfg(test)]
mod tests {

    use crate::contract::{execute, init, query};
    use cosmwasm_std::testing::{mock_dependencies, mock_info};
    use cosmwasm_std::{attr, from_binary, Decimal, HumanAddr, StdError, Uint128};

    use nebula_protocol::staking::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, PoolInfoResponse, QueryMsg,
    };

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
            terraswap_factory: HumanAddr::from("terraswap-factory"),
        };

        let env = mock_info("addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), env, msg).unwrap();

        // it worked, let's query the state
        let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
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

        let msg = InstantiateMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
            terraswap_factory: HumanAddr::from("terraswap-factory"),
        };

        let env = mock_info("addr", &[]);
        let _res = instantiate(deps.as_mut(), env.clone(), msg).unwrap();

        // update owner
        let env = mock_info("owner", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(HumanAddr("owner2".to_string())),
        };

        let res = execute(deps.as_mut(), env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(
            ConfigResponse {
                owner: HumanAddr::from("owner2"),
                nebula_token: HumanAddr::from("reward"),
            },
            config
        );

        // unauthorized err
        let env = mock_info("owner", &[]);
        let msg = ExecuteMsg::UpdateConfig { owner: None };

        let res = execute(deps.as_mut(), env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }
    }

    #[test]
    fn test_register() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InstantiateMsg {
            owner: HumanAddr::from("owner"),
            nebula_token: HumanAddr::from("reward"),
            terraswap_factory: HumanAddr::from("terraswap-factory"),
        };

        let env = mock_info("addr", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), env, msg).unwrap();

        let msg = ExecuteMsg::RegisterAsset {
            asset_token: HumanAddr::from("asset"),
            staking_token: HumanAddr::from("staking"),
        };

        // failed with unauthorized error
        let env = mock_info("addr", &[]);
        let res = execute(deps.as_mut(), env, msg.clone()).unwrap_err();
        match res {
            StdError::Unauthorized { .. } => {}
            _ => panic!("DO NOT ENTER HERE"),
        }

        let env = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "register_asset"),
                attr("asset_token", "asset"),
            ]
        );

        let res = query(
            deps.as_ref(),
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
