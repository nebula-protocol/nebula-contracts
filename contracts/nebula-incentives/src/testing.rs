use crate::contract::{handle, init, query_config};
use crate::mock_querier::{WasmMockQuerier, mock_dependencies};
use cosmwasm_std::testing::{MOCK_CONTRACT_ADDR, MockApi, MockStorage, mock_env};
use cosmwasm_std::{Coin, CosmosMsg, Decimal, Env, Extern, HumanAddr, StdError, Uint128, WasmMsg, from_binary, log, to_binary};
use cw20::{Cw20HandleMsg, Cw20ReceiveMsg};
use nebula_protocol::incentives::{ConfigResponse, Cw20HookMsg, HandleMsg, InitMsg, PoolType};
use nebula_protocol::gov::Cw20HookMsg::DepositReward;
use terraswap::asset::{Asset, AssetInfo};
use terraswap::pair::{Cw20HookMsg as TerraswapCw20HookMsg, HandleMsg as TerraswapHandleMsg};

const TEST_CREATOR: &str = "creator";

fn init_msg() -> InitMsg {
    InitMsg {
        factory: HumanAddr("factory".to_string()),
        custody: HumanAddr("custody".to_string()),
        terraswap_factory: HumanAddr("terraswap_factory".to_string()),
        nebula_token: HumanAddr("nebula_token".to_string()),
        base_denom: "uusd".to_string(),
        owner: HumanAddr("owner0000".to_string()),
    }
}

fn mock_init(mut deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    let msg = init_msg();
    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
}

fn mock_env_height(sender: &str, sent: &[Coin], height: u64, time: u64) -> Env {
    let mut env = mock_env(sender, sent);
    env.block.height = height;
    env.block.time = time;
    env
}


#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();

    let env = mock_env("owner0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    // it worked, let's query the state
    let config: ConfigResponse = query_config(&deps).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0000"),
            factory: HumanAddr("factory".to_string()),
            custody: HumanAddr("custody".to_string()),
            nebula_token: HumanAddr::from("nebula_token"),
            terraswap_factory: HumanAddr::from("terraswap_factory"),
            base_denom:"uusd".to_string(),
        }
    );

    let msg = HandleMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_env("owner0001", &[]);
    let res = handle(&mut deps, env, msg);

    match res {
        Ok(_) => panic!("Must return error"),
        Err(e) => assert_eq!(e, StdError::unauthorized()),
    }

    let msg = HandleMsg::UpdateOwner {
        owner: HumanAddr::from("owner0001"),
    };

    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "update_owner"),
        ]
    );

    // it worked, let's query the state
    let config: ConfigResponse = query_config(&deps).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: HumanAddr::from("owner0001"),
            factory: HumanAddr("factory".to_string()),
            custody: HumanAddr("custody".to_string()),
            nebula_token: HumanAddr::from("nebula_token"),
            terraswap_factory: HumanAddr::from("terraswap_factory"),
            base_denom:"uusd".to_string(),
        }
    );
}

#[test]
fn test_deposit_reward() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);

    let rewards_amount = Uint128(1000);
    let total_rewards_amount = Uint128(2000);

    // Send Nebula token to this contract
    let msg = HandleMsg::Receive(Cw20ReceiveMsg {
        sender: HumanAddr::from(TEST_CREATOR),
        amount: total_rewards_amount,
        msg: Some(
            to_binary(&Cw20HookMsg::DepositReward {
                rewards: vec![
                    (PoolType::REBALANCER, HumanAddr::from("asset"), rewards_amount),
                    (PoolType::ARBITRAGER, HumanAddr::from("asset"), rewards_amount)],
            })
            .unwrap(),
        ),
    });
    let env = mock_env(HumanAddr::from("nebula_token"), &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "deposit_reward"),
            log("amount", total_rewards_amount),
        ]
    );
}

#[test]
fn test_penalty_period() {
    let mut deps = mock_dependencies(20, &[]);

    mock_init(&mut deps);
    let msg = HandleMsg::NewPenaltyPeriod {};
    let env = mock_env("owner0000", &[]);
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();

    assert_eq!(
        res.log,
        vec![
            log("action", "new_penalty_period"),
            log("previous_n", 0),
            log("current_n", 1)
        ]
    );
}

// TODO: Integration test with deposit reward, penalty period, withdraw reward + queries

// TODO: Integration for mint / redeem

// TODO: Specific math tests for cluster_imbalance, terraswap imbalance

// TODO: Specific tests for internal function (SendAll, SwapAll, InternalRewarded*, RecordRewards)

// TODO: Integration for arbcluster / mint