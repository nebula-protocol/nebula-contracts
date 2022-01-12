use std::str::FromStr;

use crate::contract::{execute, instantiate, query};
use crate::contract::{get_ema, notional_penalty, update_ema};
use crate::mock_querier::mock_dependencies;
use crate::state::{read_config, PenaltyConfig};

use cluster_math::{
    dot, imbalance, int32_vec_to_fpdec, int_vec_to_fpdec, str_vec_to_fpdec, FPDecimal,
};
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, from_binary, Addr, DepsMut, Env, StdError, Timestamp, Uint128};
use nebula_protocol::penalty::{
    ExecuteMsg, InstantiateMsg, PenaltyCreateResponse, PenaltyParams, PenaltyRedeemResponse,
    QueryMsg,
};

const TEST_CREATOR: &str = "creator";

fn mock_init(deps: DepsMut) {
    let msg = InstantiateMsg {
        owner: TEST_CREATOR.to_string(),
        penalty_params: init_params(),
    };

    let info = mock_info(TEST_CREATOR, &[]);
    let _res = instantiate(deps, mock_env(), info, msg)
        .expect("contract successfully executes InstantiateMsg");
}

fn mock_env_height(height: u64, time: u64) -> Env {
    let mut env = mock_env();
    env.block.height = height;
    env.block.time = Timestamp::from_seconds(time);
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

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner: "penalty_owner".to_string(),
        penalty_params: init_params(),
    }
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let msg = init_msg();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config: PenaltyConfig = read_config(deps.as_mut().storage).unwrap();
    assert_eq!(
        config,
        PenaltyConfig {
            owner: Addr::unchecked("penalty_owner"),
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
    let mut deps = mock_dependencies(&[]);
    let msg = init_msg();
    let info = mock_info("addr0000", &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let ema = get_ema(deps.as_ref(), 50, FPDecimal::from(100u128)).unwrap();

    // Should return net asset value if not updated at all
    let res = FPDecimal::from_str("100").unwrap();
    assert_eq!(res, ema);

    // Check EMA 70 blocks in the future
    update_ema(deps.as_mut(), 50, FPDecimal::from(100u128)).unwrap();
    let ema = get_ema(deps.as_ref(), 120, FPDecimal::from(120u128)).unwrap();
    let res = FPDecimal::from_str("102.2023645802395236").unwrap();
    assert_eq!(res, ema);
}

#[test]
fn test_notional_penalty_math() {
    let mut deps = mock_dependencies(&[]);

    let msg = init_msg();
    let info = mock_info("addr0000", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Test penalty but with no imbalance is too high error
    let i0 = int32_vec_to_fpdec(&[95, 100, 105]);
    let w = int32_vec_to_fpdec(&[100, 100, 100]);
    let p = str_vec_to_fpdec(&["8.7".to_string(), "2.1".to_string(), "3.5".to_string()]).unwrap();

    let i1 = int32_vec_to_fpdec(&[90, 100, 110]);
    let penalty = notional_penalty(deps.as_ref(), 0u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("-32.747139223416719612").unwrap();
    assert_eq!(res, penalty);

    // Test penalty but now imbalance is too high
    let i1 = int32_vec_to_fpdec(&[80, 100, 120]);
    let penalty = notional_penalty(deps.as_ref(), 0u64, &i0, &i1, &w, &p);

    match penalty {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "cluster imbalance too high"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // Test reward by correcting imbalance
    let i1 = int32_vec_to_fpdec(&[98, 100, 102]);
    let reward = notional_penalty(deps.as_ref(), 0u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("1.36418181815").unwrap();
    assert_eq!(res, reward);

    //// Try everything again smaller nav and updated_ema
    let curr_inv = int32_vec_to_fpdec(&[47, 50, 53]);
    let nav = dot(&curr_inv, &p);
    update_ema(deps.as_mut(), 60, nav).unwrap();

    let i1 = int32_vec_to_fpdec(&[90, 100, 110]);
    let penalty = notional_penalty(deps.as_ref(), 160u64, &i0, &i1, &w, &p);
    match penalty {
        Ok(_) => panic!("Must return error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "cluster imbalance too high"),
        Err(e) => panic!("Unexpected error: {:?}", e),
    };

    let i1 = int32_vec_to_fpdec(&[95, 100, 108]);
    let penalty = notional_penalty(deps.as_ref(), 160u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("-9.769495573051444318").unwrap();
    assert_eq!(res, penalty);

    let i1 = int32_vec_to_fpdec(&[102, 100, 96]);
    let reward = notional_penalty(deps.as_ref(), 160u64, &i0, &i1, &w, &p).unwrap();
    let res = FPDecimal::from_str("1.235034965").unwrap();
    assert_eq!(res, reward);
}

#[test]
fn test_mint_actions() {
    let mut deps = mock_dependencies(&[]);
    mock_init(deps.as_mut());
    let env = mock_env_height(0, 10000);
    let info = mock_info(TEST_CREATOR, &[]);

    // Target weights and prices
    let p_strs = &["8.7".to_string(), "2.1".to_string(), "3.5".to_string()];
    let weights = &[Uint128::new(100), Uint128::new(100), Uint128::new(100)];

    let p = str_vec_to_fpdec(p_strs).unwrap();

    // Set up EMA
    let curr_inv = &[Uint128::new(1000), Uint128::new(1010), Uint128::new(994)];

    let nav = dot(&int_vec_to_fpdec(curr_inv), &p);
    update_ema(deps.as_mut(), 60, nav).unwrap();

    let create_asset_amounts = &[Uint128::new(1000), Uint128::new(1010), Uint128::new(994)];

    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::PenaltyQueryCreate {
            block_height: 120,
            cluster_token_supply: Uint128::new(1000000),
            inventory: curr_inv.to_vec(),
            create_asset_amounts: create_asset_amounts.to_vec(),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: PenaltyCreateResponse = from_binary(&res).unwrap();
    assert_eq!(response.create_tokens, Uint128::new(999706));
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.attributes, vec![attr("penalty", "-4.2")]);

    // Simulate target weights changing dramatically
    let weights = &[Uint128::new(200), Uint128::new(100), Uint128::new(100)];
    let create_asset_amounts = &[Uint128::new(3000), Uint128::new(990), Uint128::new(1006)];

    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::PenaltyQueryCreate {
            block_height: 120,
            cluster_token_supply: Uint128::new(1000000),
            inventory: curr_inv.to_vec(),
            create_asset_amounts: create_asset_amounts.to_vec(),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: PenaltyCreateResponse = from_binary(&res).unwrap();

    assert_eq!(response.create_tokens, Uint128::new(2230596));
    assert_eq!(response.penalty, Uint128::new(197));
    assert_eq!(response.attributes, vec![attr("penalty", "197.5260869565")]);

    let weights = &[Uint128::new(100), Uint128::new(100), Uint128::new(100)];
    let create_asset_amounts = &[Uint128::new(1000), Uint128::new(1010), Uint128::new(994)];
    let curr_inv = &[Uint128::new(2000), Uint128::new(2020), Uint128::new(1988)];
    let msg = ExecuteMsg::PenaltyCreate {
        block_height: 120,
        cluster_token_supply: Uint128::new(1000000),
        inventory: curr_inv.to_vec(),
        create_asset_amounts: create_asset_amounts.to_vec(),
        asset_prices: p_strs.to_vec(),
        target_weights: weights.to_vec(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    for log in res.attributes.iter() {
        match log.key.as_str() {
            "new_ema" => assert_eq!("15660.8249220857780489", log.value),
            &_ => panic!("Invalid value found in log"),
        }
    }
}

#[test]
fn test_redeem_actions() {
    let mut deps = mock_dependencies(&[]);
    mock_init(deps.as_mut());
    let env = mock_env_height(0, 10000);

    // Target weights and prices
    let p_strs = &["8.7".to_string(), "2.1".to_string(), "3.5".to_string()];
    let weights = &[Uint128::new(100), Uint128::new(100), Uint128::new(100)];

    let p = str_vec_to_fpdec(p_strs).unwrap();

    // Set up EMA
    let curr_inv = &[Uint128::new(1000), Uint128::new(1010), Uint128::new(994)];

    let nav = dot(&int_vec_to_fpdec(curr_inv), &p);
    update_ema(deps.as_mut(), 60, nav).unwrap();

    let redeem_asset_amounts = &[Uint128::new(100), Uint128::new(100), Uint128::new(100)];

    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::PenaltyQueryRedeem {
            block_height: 120,
            cluster_token_supply: Uint128::new(1000000),
            inventory: curr_inv.to_vec(),
            redeem_asset_amounts: redeem_asset_amounts.to_vec(),
            max_tokens: Uint128::new(10000),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: PenaltyRedeemResponse = from_binary(&res).unwrap();
    assert_eq!(response.token_cost, Uint128::new(100000));
    assert_eq!(
        response.redeem_assets,
        vec![Uint128::new(100), Uint128::new(100), Uint128::new(100)]
    );
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.attributes, vec![attr("penalty", "0")]);

    let redeem_asset_amounts = &[];

    let res = query(
        deps.as_ref(),
        env.clone(),
        QueryMsg::PenaltyQueryRedeem {
            block_height: 120,
            cluster_token_supply: Uint128::new(1000000),
            inventory: curr_inv.to_vec(),
            redeem_asset_amounts: redeem_asset_amounts.to_vec(),
            max_tokens: Uint128::new(100000),
            asset_prices: p_strs.to_vec(),
            target_weights: weights.to_vec(),
        },
    )
    .unwrap();

    let response: PenaltyRedeemResponse = from_binary(&res).unwrap();
    assert_eq!(response.token_cost, Uint128::new(100000));
    assert_eq!(
        response.redeem_assets,
        vec![Uint128::new(100), Uint128::new(101), Uint128::new(99)]
    );
    assert_eq!(response.penalty, Uint128::zero());
    assert_eq!(response.attributes.len(), 0usize);

    let weights = &[Uint128::new(100), Uint128::new(100), Uint128::new(100)];
    let create_asset_amounts = &[Uint128::new(1000), Uint128::new(1010), Uint128::new(994)];
    let curr_inv = &[Uint128::new(900), Uint128::new(910), Uint128::new(904)];
    let msg = ExecuteMsg::PenaltyRedeem {
        block_height: 120,
        cluster_token_supply: Uint128::new(1000000),
        inventory: curr_inv.to_vec(),
        redeem_asset_amounts: create_asset_amounts.to_vec(),
        max_tokens: Uint128::new(10000),
        asset_prices: p_strs.to_vec(),
        target_weights: weights.to_vec(),
    };

    let info = mock_info(TEST_CREATOR, &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    for log in res.attributes.iter() {
        match log.key.as_str() {
            "new_ema" => assert_eq!("14167.248198160163609915", log.value),
            &_ => panic!("Invalid value found in log"),
        }
    }
}
