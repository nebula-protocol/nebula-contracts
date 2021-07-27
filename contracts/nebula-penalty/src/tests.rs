use std::str::FromStr;

use crate::contract::{get_ema, notional_penalty, update_ema};
use crate::contract::{handle, init, query};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{read_config, PenaltyConfig};

use cluster_math::{imbalance, int32_vec_to_fpdec, str_vec_to_fpdec, FPDecimal};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, log, to_binary, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HumanAddr, StdError, StdResult, Uint128, WasmMsg,
};
use nebula_protocol::penalty::{
    HandleMsg, InitMsg, MintResponse, ParamsResponse, PenaltyParams, RedeemResponse,
};

const TEST_CREATOR: &str = "creator";

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

#[test]
fn test_imbalance() {
    let i = int32_vec_to_fpdec(&[10, 10, 10]);
    let p = str_vec_to_fpdec(&["8.7".to_string(), "2.1".to_string(), "3.5".to_string()]).unwrap();
    let w = int32_vec_to_fpdec(&[10, 10, 10]);

    let imb = imbalance(&i, &p, &w);
    assert_eq!(FPDecimal::zero(), imb);

    let i = int32_vec_to_fpdec(&[5, 10, 15]);
    let imb = imbalance(&i, &p, &w);
    println!("{}", imb);
    let res = FPDecimal::from_str("55.363636363636363636").unwrap();
    assert_eq!(res, imb);
}

#[test]
fn test_ema_math() {
    let mut deps = mock_dependencies(20, &[]);
    let msg = init_msg();
    let env = mock_env("addr0000", &[]);
    init(&mut deps, env, msg).unwrap();
    let ema = get_ema(&mut deps, 50, FPDecimal::from(100u128)).unwrap();

    // Should return net asset value if not updated at all
    let res = FPDecimal::from_str("100").unwrap();
    assert_eq!(res, ema);

    update_ema(&mut deps, 50, FPDecimal::from(100u128)).unwrap();
    let ema = get_ema(&deps, 120, FPDecimal::from(120u128)).unwrap();
    let res = FPDecimal::from_str("102.2023645802395236").unwrap();
    assert_eq!(res, ema);
}

#[test]
fn test_notional_penalty_math() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = init_msg();
    let env = mock_env("addr0000", &[]);
    let res = init(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let i0 = int32_vec_to_fpdec(&[95, 100, 105]);
    let i1 = int32_vec_to_fpdec(&[90, 100, 110]);
    let w = int32_vec_to_fpdec(&[100, 100, 100]);
    let p = str_vec_to_fpdec(&["8.7".to_string(), "2.1".to_string(), "3.5".to_string()]).unwrap();
    let penalty = notional_penalty(&deps, 0u64, &i0, &i1, &w, &p).unwrap();
    println!("mad or naw, YAH {}", penalty);

    // let i0 = int32_vec_to_fpdec(&[9, 10, 11]);
    // let i1 = int32_vec_to_fpdec(&[5, 10, 15]);
    // let w = int32_vec_to_fpdec(&[10, 10, 10]);
    // let p = str_vec_to_fpdec(&["8.7".to_string(), "2.1".to_string(), "3.5".to_string()]).unwrap();
    
    // let penalty = notional_penalty(&deps, 0u64, &i0, &i1, &w, &p);
    // println!("{}", penalty.unwrap());
}
