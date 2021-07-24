use std::str::FromStr;

use crate::contract::{handle, init, query};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{PenaltyConfig, read_config};

use cluster_math::FPDecimal;
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse, HumanAddr, StdError, StdResult, Uint128, WasmMsg, coins, from_binary, log, to_binary};
use nebula_protocol::penalty::{
    HandleMsg, InitMsg, MintResponse, RedeemResponse, ParamsResponse, PenaltyParams
};

const VOTING_TOKEN: &str = "voting_token";
const TEST_CREATOR: &str = "creator";
const TEST_VOTER: &str = "voter1";
const TEST_VOTER_2: &str = "voter2";
const TEST_VOTER_3: &str = "voter3";
const TEST_COLLECTOR: &str = "collector";
const DEFAULT_QUORUM: u64 = 30u64;
const DEFAULT_THRESHOLD: u64 = 50u64;
const DEFAULT_VOTING_PERIOD: u64 = 10000u64;
const DEFAULT_EFFECTIVE_DELAY: u64 = 10000u64;
const DEFAULT_EXPIRATION_PERIOD: u64 = 20000u64;
const DEFAULT_PROPOSAL_DEPOSIT: u128 = 10000000000u128;
const DEFAULT_VOTER_WEIGHT: Decimal = Decimal::zero();
const DEFAULT_SNAPSHOT_PERIOD: u64 = 10u64;

fn mock_init(mut deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    let msg = InitMsg {
        owner: HumanAddr::from("penalty_owner"),
        penalty_params: init_params(),
    };

    let env = mock_env(TEST_CREATOR, &[]);
    let _res = init(&mut deps, env, msg).expect("contract successfully handles InitMsg");
}

fn mock_env_height(sender: &str, sent: &[Coin], height: u64, time: u64) -> Env {
    let mut env = mock_env(sender, sent);
    env.block.height = height;
    env.block.time = time;
    env
}

fn init_params() -> PenaltyParams {
    PenaltyParams {
        penalty_amt_lo: FPDecimal::from_str("0.1").unwrap(),
        penalty_cutoff_lo: FPDecimal::from_str("0.01").unwrap(),
        penalty_amt_hi: FPDecimal::from_str("1").unwrap(),
        penalty_cutoff_hi: FPDecimal::from_str("0.1").unwrap(),
        reward_amt: FPDecimal::from_str("0.05").unwrap(),
        reward_cutoff: FPDecimal::from_str("0.02").unwrap(),
    }
}

fn init_msg() -> InitMsg {
    InitMsg {
        owner: HumanAddr::from("penalty_owner"),
        penalty_params: init_params(),
    }
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();
    let env = mock_env("addr0000", &[]);
    let res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config: PenaltyConfig = read_config(&mut deps.storage).unwrap();
    assert_eq!(
        config,
        PenaltyConfig {
            owner: HumanAddr::from("penalty_owner"),
            penalty_params: PenaltyParams {
                penalty_amt_lo: FPDecimal::from_str("0.1").unwrap(),
                penalty_cutoff_lo: FPDecimal::from_str("0.01").unwrap(),
                penalty_amt_hi: FPDecimal::from_str("1").unwrap(),
                penalty_cutoff_hi: FPDecimal::from_str("0.1").unwrap(),
                reward_amt: FPDecimal::from_str("0.05").unwrap(),
                reward_cutoff: FPDecimal::from_str("0.02").unwrap(),
            },
            ema: FPDecimal::zero(), 
            last_block: 0u64,
        }
    );
}
