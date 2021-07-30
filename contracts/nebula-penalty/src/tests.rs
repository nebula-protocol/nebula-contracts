use std::str::FromStr;

use crate::contract::{get_ema, notional_penalty, update_ema};
use crate::contract::{handle, init, query};
use crate::mock_querier::{mock_dependencies, WasmMockQuerier};
use crate::state::{read_config, PenaltyConfig};

use cluster_math::{FPDecimal, dot, imbalance, int32_vec_to_fpdec, int_vec_to_fpdec, str_vec_to_fpdec};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coins, from_binary, log, to_binary, Coin, CosmosMsg, Decimal, Env, Extern, HandleResponse,
    HumanAddr, StdError, StdResult, Uint128, WasmMsg,
};
use nebula_protocol::penalty::{HandleMsg, InitMsg, MintResponse, ParamsResponse, PenaltyParams, QueryMsg, RedeemResponse};

const TEST_CREATOR: &str = "creator";

fn mock_init(mut deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    let msg = InitMsg {
        owner: HumanAddr::from(TEST_CREATOR),
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

    // Check EMA 70 blocks in the future
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

    // Test penalty but with no imbalance is too high error
    let i0 = int32_vec_to_fpdec(&[95, 100, 105]);
    let w = int32_vec_to_fpdec(&[100, 100, 100]);
    let p = str_vec_to_fpdec(&["8.7".to_string(), "2.1".to_string(), "3.5".to_string()]).unwrap();

    let i1 = int32_vec_to_fpdec(&[90, 100, 110]);
    let penalty = notional_penalty(&deps, 0u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("-32.747139223416719612").unwrap();
    assert_eq!(res, penalty);

    // Test penalty but now imbalance is too high
    let i1 = int32_vec_to_fpdec(&[80, 100, 120]);
    let penalty = notional_penalty(&deps, 0u64, &i0, &i1, &w, &p);

    match penalty {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "cluster imbalance too high"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // Test reward by correcting imbalance
    let i1 = int32_vec_to_fpdec(&[98, 100, 102]);
    let reward = notional_penalty(&deps, 0u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("1.36418181815").unwrap();
    assert_eq!(res, reward);

    //// Try everything again smaller nav and updated_ema
    let curr_inv = int32_vec_to_fpdec(&[47, 50, 53]);
    let nav = dot(&curr_inv, &p);
    update_ema(&mut deps, 60, nav).unwrap();
    
    let i1 = int32_vec_to_fpdec(&[90, 100, 110]);
    let penalty = notional_penalty(&deps, 160u64, &i0, &i1, &w, &p);
    match penalty {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "cluster imbalance too high"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    };

    let i1 = int32_vec_to_fpdec(&[95, 100, 108]);
    let penalty = notional_penalty(&deps, 160u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("-9.769495573051444318").unwrap();
    assert_eq!(res, penalty);

    let i1 = int32_vec_to_fpdec(&[102, 100, 96]);
    let reward = notional_penalty(&deps, 160u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("1.235034965").unwrap();
    assert_eq!(res, reward);
}

#[test]
fn test_mint_actions() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(TEST_CREATOR, &vec![], 0, 10000);

    // Target weights and prices
    let p_strs = &["8.7".to_string(), "2.1".to_string(), "3.5".to_string()];
    let weights = &[Uint128(100), Uint128(100), Uint128(100)];

    let p = str_vec_to_fpdec(p_strs).unwrap();

    // Set up EMA
    let curr_inv = &[Uint128(1000), Uint128(1010), Uint128(994)];
    
    let nav = dot(&int_vec_to_fpdec(curr_inv), &p);
    update_ema(&mut deps, 60, nav).unwrap();

    let mint_asset_amounts = &[Uint128(1000), Uint128(1010), Uint128(994)];

    let res = query(
        &deps,
        QueryMsg::Mint {
            block_height: 120,
            cluster_token_supply: Uint128(1000000),
            inventory: curr_inv.to_vec(),
            mint_asset_amounts: mint_asset_amounts.to_vec(),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: MintResponse = from_binary(&res).unwrap();
    assert_eq!(response.mint_tokens, Uint128(999706));
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.log, vec![log("penalty", FPDecimal::from_str("-4.2").unwrap())]);

    // Simulate target weights changing dramatically
    let weights = &[Uint128(200), Uint128(100), Uint128(100)];
    let mint_asset_amounts = &[Uint128(3000), Uint128(990), Uint128(1006)];

    let res = query(
        &deps,
        QueryMsg::Mint {
            block_height: 120,
            cluster_token_supply: Uint128(1000000),
            inventory: curr_inv.to_vec(),
            mint_asset_amounts: mint_asset_amounts.to_vec(),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();


    let response: MintResponse = from_binary(&res).unwrap();


    assert_eq!(response.mint_tokens, Uint128(2230596));
    assert_eq!(response.penalty, Uint128(197));
    assert_eq!(response.log, vec![log("penalty", FPDecimal::from_str("197.5260869565").unwrap())]);


    let weights = &[Uint128(100), Uint128(100), Uint128(100)];
    let mint_asset_amounts = &[Uint128(1000), Uint128(1010), Uint128(994)];
    let curr_inv = &[Uint128(2000), Uint128(2020), Uint128(1988)];
    let msg = HandleMsg::Mint { 
        block_height: 120, 
        cluster_token_supply: Uint128(1000000),
        inventory: curr_inv.to_vec(), 
        mint_asset_amounts: mint_asset_amounts.to_vec(), 
        asset_prices: p_strs.to_vec(), 
        target_weights: weights.to_vec()
    };

    let res = handle(&mut deps, env, msg).unwrap();
    for log in res.log.iter() {
        match log.key.as_str() {
            "new_ema" => assert_eq!("15660.8249220857780489", log.value),
            &_ => panic!("Invalid value found in log")
        }
    }
}

#[test]
fn test_redeem_actions() {
    let mut deps = mock_dependencies(20, &[]);
    mock_init(&mut deps);
    let env = mock_env_height(TEST_CREATOR, &vec![], 0, 10000);

    // Target weights and prices
    let p_strs = &["8.7".to_string(), "2.1".to_string(), "3.5".to_string()];
    let weights = &[Uint128(100), Uint128(100), Uint128(100)];

    let p = str_vec_to_fpdec(p_strs).unwrap();

    // Set up EMA
    let curr_inv = &[Uint128(1000), Uint128(1010), Uint128(994)];
    
    let nav = dot(&int_vec_to_fpdec(curr_inv), &p);
    update_ema(&mut deps, 60, nav).unwrap();

    let redeem_asset_amounts = &[Uint128(100), Uint128(100), Uint128(100)];

    let res = query(
        &deps,
        QueryMsg::Redeem {
            block_height: 120,
            cluster_token_supply: Uint128(1000000),
            inventory: curr_inv.to_vec(),
            redeem_asset_amounts: redeem_asset_amounts.to_vec(),
            max_tokens: Uint128(10000),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: RedeemResponse = from_binary(&res).unwrap();
    assert_eq!(response.token_cost, Uint128(100000));
    assert_eq!(response.redeem_assets, vec![Uint128(100), Uint128(100), Uint128(100)]);
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.log, vec![log("penalty", FPDecimal::from_str("0").unwrap())]);

    let redeem_asset_amounts = &[];

    let res = query(
        &deps,
        QueryMsg::Redeem {
            block_height: 120,
            cluster_token_supply: Uint128(1000000),
            inventory: curr_inv.to_vec(),
            redeem_asset_amounts: redeem_asset_amounts.to_vec(),
            max_tokens: Uint128(100000),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: RedeemResponse = from_binary(&res).unwrap();
    assert_eq!(response.token_cost, Uint128(100000));
    assert_eq!(response.redeem_assets, vec![Uint128(100), Uint128(101), Uint128(99)]);
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.log, vec![]);


    let weights = &[Uint128(100), Uint128(100), Uint128(100)];
    let mint_asset_amounts = &[Uint128(1000), Uint128(1010), Uint128(994)];
    let curr_inv = &[Uint128(900), Uint128(910), Uint128(904)];
    let msg = HandleMsg::Redeem { 
        block_height: 120, 
        cluster_token_supply: Uint128(1000000),
        inventory: curr_inv.to_vec(), 
        redeem_asset_amounts: mint_asset_amounts.to_vec(),
        max_tokens: Uint128(10000),
        asset_prices: p_strs.to_vec(), 
        target_weights: weights.to_vec()
    };

    let res = handle(&mut deps, env, msg).unwrap();
    for log in res.log.iter() {
        match log.key.as_str() {
            "new_ema" => assert_eq!("14167.248198160163609915", log.value),
            &_ => panic!("Invalid value found in log")
        }
    }
}