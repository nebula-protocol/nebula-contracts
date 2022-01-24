use crate::contract::{execute, instantiate, query};
use crate::state::{read_config, Config};
use crate::testing::mock_querier::mock_dependencies;
use astroport::asset::AssetInfo;
use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{from_binary, Addr, Decimal, StdError};
use nebula_protocol::oracle::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};

fn init_msg() -> InstantiateMsg {
    InstantiateMsg {
        owner: "owner0000".to_string(),
        oracle_addr: "oracle0000".to_string(),
        base_denom: "uusd".to_string(),
    }
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("sender0000", &[]);
    let msg = init_msg();
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config: Config = read_config(&deps.storage).unwrap();
    assert_eq!(
        config,
        Config {
            owner: Addr::unchecked("owner0000"),
            oracle_addr: Addr::unchecked("oracle0000"),
            base_denom: "uusd".to_string(),
        }
    );
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("sender0000", &[]);
    let msg = init_msg();
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // unauthorized update
    let info = mock_info("imposter0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("imposter0000".to_string()),
        oracle_addr: Some("oracle0001".to_string()),
        base_denom: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized"));

    // successful update
    let info = mock_info("owner0000", &[]);
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some("owner0001".to_string()),
        oracle_addr: Some("oracle0001".to_string()),
        base_denom: Some("uusd".to_string()),
    };
    let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    let config = read_config(&deps.storage).unwrap();
    assert_eq!(
        config,
        Config {
            owner: Addr::unchecked("owner0001"),
            oracle_addr: Addr::unchecked("oracle0001"),
            base_denom: "uusd".to_string()
        }
    )
}

#[test]
fn query_price() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("sender0000", &[]);
    let msg = init_msg();
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let msg = QueryMsg::Price {
        base_asset: AssetInfo::Token {
            contract_addr: Addr::unchecked("contractaddress0001"),
        },
        quote_asset: AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
    };
    let res = query(deps.as_ref(), mock_env(), msg).unwrap();
    let price: PriceResponse = from_binary(&res).unwrap();
    assert_eq!(price.rate, Decimal::one());
}
