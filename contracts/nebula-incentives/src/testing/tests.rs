use crate::contract::{execute, instantiate, migrate, query};
use crate::error::ContractError;
use crate::query::query_config;
use crate::state::{contributions_read, read_from_contribution_bucket, record_contribution};
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::{Asset, AssetInfo};
use astroport::pair::PoolResponse as AstroportPoolResponse;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, CosmosMsg, DepsMut, SubMsg, Uint128, WasmMsg,
};
use cw2::{get_contract_version, ContractVersion};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use nebula_protocol::incentives::{
    ConfigResponse, ContributorPendingRewardsResponse, CurrentContributorInfoResponse, Cw20HookMsg,
    ExecuteMsg, IncentivesPoolInfoResponse, InstantiateMsg, MigrateMsg, PenaltyPeriodResponse,
    PoolType, QueryMsg,
};

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        proxy: ("proxy".to_string()),
        custody: ("custody".to_string()),
        nebula_token: ("nebula_token".to_string()),
        owner: ("owner0000".to_string()),
    }
}

fn mock_init(deps: DepsMut) {
    let msg = init_msg();
    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps, mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = init_msg();

    let info = mock_info("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    // it worked, let's query the state
    let msg = QueryMsg::Config {};
    let config: ConfigResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0000".to_string(),
            proxy: "proxy".to_string(),
            custody: "custody".to_string(),
            nebula_token: "nebula_token".to_string(),
        }
    );

    let msg = ExecuteMsg::UpdateConfig {
        owner: "owner0001".to_string(),
    };

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let msg = ExecuteMsg::UpdateConfig {
        owner: "owner0001".to_string(),
    };

    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.attributes, vec![attr("action", "update_config"),]);

    // it worked, let's query the state
    let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "owner0001".to_string(),
            proxy: "proxy".to_string(),
            custody: "custody".to_string(),
            nebula_token: "nebula_token".to_string(),
        }
    );
}

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: total_rewards_amount,
        msg: to_binary(&Cw20HookMsg::DepositReward {
            rewards: vec![
                (PoolType::REBALANCE, "cluster".to_string(), rewards_amount),
                (PoolType::ARBITRAGE, "cluster".to_string(), rewards_amount),
            ],
        })
        .unwrap(),
    });
    let info = mock_info("nebula_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "nebula_token".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "custody".to_string(),
                amount: total_rewards_amount,
            })
            .unwrap(),
            funds: vec![],
        }))]
    );

    // now we can query the pool info with the new reward amount
    let msg = QueryMsg::PoolInfo {
        pool_type: PoolType::REBALANCE,
        cluster_address: "cluster".to_string(),
        n: None,
    };
    let res: IncentivesPoolInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        IncentivesPoolInfoResponse {
            value_total: Uint128::zero(),
            reward_total: Uint128::new(1000),
        }
    );
}

#[test]
fn test_penalty_period() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());
    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "new_penalty_period"),
            attr("previous_n", "0"),
            attr("current_n", "1")
        ]
    );

    let res = query(deps.as_ref(), mock_env(), QueryMsg::PenaltyPeriod {}).unwrap();
    let response: PenaltyPeriodResponse = from_binary(&res).unwrap();
    assert_eq!(response, PenaltyPeriodResponse { n: 1 });
}

#[test]
fn test_withdraw_reward() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    // First, deposit rewards for both pools
    let rewards_amount = Uint128::new(1000);
    let total_rewards_amount = Uint128::new(2000);

    let deposit_msg = to_binary(&Cw20HookMsg::DepositReward {
        rewards: vec![
            (PoolType::REBALANCE, "cluster".to_string(), rewards_amount),
            (PoolType::ARBITRAGE, "cluster".to_string(), rewards_amount),
        ],
    })
    .unwrap();

    // Specify wrong reward amount
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: Uint128::new(500),
        msg: deposit_msg.clone(),
    });
    let info = mock_info("nebula_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(
        res,
        ContractError::Generic("Rewards amount miss matched".to_string())
    );

    // Send wrong token to this contract
    let info = mock_info("wrong_token", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    // Send Nebula token to this contract
    let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
        sender: TEST_CREATOR.to_string(),
        amount: total_rewards_amount,
        msg: deposit_msg,
    });
    let info = mock_info("nebula_token", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Manually record contribution to pools; one pool has other contribution from another address, make sure ratio is correct
    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0000"),
        PoolType::REBALANCE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    // try querying contribution info
    let msg = QueryMsg::CurrentContributorInfo {
        pool_type: PoolType::REBALANCE,
        contributor_address: "contributor0000".to_string(),
        cluster_address: "cluster".to_string(),
    };
    let res: CurrentContributorInfoResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(
        res,
        CurrentContributorInfoResponse {
            n: 0,
            value_contributed: Uint128::new(25)
        }
    );

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0000"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0001"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    // Test without advancing penalty period (should give 0)

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "0"),]
    );

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0000".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::zero());

    // Advance penalty period

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    // Now, we are eligible to collect rewards

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0000".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::from(1500u128));

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "1500"),]
    );

    // Overwrite the old contribution in the previos penalty periond
    // and move old rewards to `pending_rewards`

    record_contribution(
        deps.as_mut(),
        &Addr::unchecked("contributor0001"),
        PoolType::ARBITRAGE,
        &Addr::unchecked("cluster"),
        Uint128::new(25),
    )
    .unwrap();

    let msg = QueryMsg::ContributorPendingRewards {
        contributor_address: "contributor0001".to_string(),
    };
    let res: ContributorPendingRewardsResponse =
        from_binary(&query(deps.as_ref(), mock_env(), msg).unwrap()).unwrap();
    assert_eq!(res.pending_rewards, Uint128::from(500u128));

    let msg = ExecuteMsg::Withdraw {};
    let info = mock_info("contributor0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.attributes,
        vec![attr("action", "withdraw"), attr("amount", "500"),]
    );
}

/// Integration tests for recording operations

#[test]
fn test_record_rebalancer_rewards() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());
    let msg = ExecuteMsg::NewPenaltyPeriod {};

    // unauthorized
    let info = mock_info("imposter0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::RecordRebalancerRewards {
        cluster_contract: Addr::unchecked("cluster"),
        rebalancer: Addr::unchecked("rebalancer"),
        // actually not used
        original_inventory: vec![Uint128::new(120), Uint128::new(100), Uint128::new(80)],
    };

    // unauthorized, sender of the contribution is not Proxy
    let info = mock_info("imposter0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("proxy", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_rebalancer_rewards"),
            attr("rebalancer_imbalance_fixed", "49"),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        &deps.storage,
        &Addr::unchecked("rebalancer"),
        PoolType::REBALANCE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &Addr::unchecked("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(49));
}

#[test]
fn test_record_astroport_impact() {
    let mut deps = mock_dependencies(&[]);

    mock_init(deps.as_mut());

    deps.querier.with_astroport_pairs(&[(
        &"uusdcluster_token".to_string(),
        &"uusd_cluster_pair".to_string(),
    )]);

    let msg = ExecuteMsg::NewPenaltyPeriod {};
    let info = mock_info("owner0000", &[]);
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    let msg = ExecuteMsg::RecordAstroportImpact {
        cluster_contract: Addr::unchecked("cluster"),
        arbitrageur: Addr::unchecked("arbitrageur"),
        astroport_pair: Addr::unchecked("uusd_cluster_pair"),
        pool_before: AstroportPoolResponse {
            assets: [
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("cluster_token"),
                    },
                    amount: Uint128::new(100),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uusd".to_string(),
                    },
                    amount: Uint128::new(1000),
                },
            ],
            total_share: Uint128::new(100),
        },
    };

    // unauthorized, sender of the contribution is not Proxy
    let info = mock_info("imposter0000", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    let info = mock_info("proxy", &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "record_astroport_arbitrageur_rewards"),
            attr("fair_value", "1.6345"),
            attr("arbitrage_imbalance_fixed", "567.862934322973128547"),
            attr("arbitrage_imbalance_sign", "1"),
            attr("imb0", "595.710499796127160136"),
            attr("imb1", "27.847565473154031589"),
        ]
    );

    // See if stateful changes actually happens
    let contribution_bucket = contributions_read(
        &deps.storage,
        &Addr::unchecked("arbitrageur"),
        PoolType::ARBITRAGE,
    );
    let contribution =
        read_from_contribution_bucket(&contribution_bucket, &Addr::unchecked("cluster"));

    assert_eq!(contribution.n, 1);
    assert_eq!(contribution.value_contributed, Uint128::new(567));
}

#[test]
fn migration() {
    let mut deps = mock_dependencies(&[]);
    mock_init(deps.as_mut());

    // assert contract infos
    assert_eq!(
        get_contract_version(&deps.storage),
        Ok(ContractVersion {
            contract: "nebula-incentives".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string()
        })
    );

    // let's migrate the contract
    let msg = MigrateMsg {};

    // we can just call .unwrap() to assert this was a success
    let _res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
}
