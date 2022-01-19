use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use crate::state::{Config, read_config};
use cosmwasm_std::testing::{mock_dependencies, mock_info, mock_env};


#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info("sender0000", &[]);
    let msg = InstantiateMsg { owner: "owner0000".to_string() };
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    let config: Config = read_config(&deps.storage).unwrap();
    assert_eq!(
        config,
        Config {
            owner: "owner0000".to_string(),
        }
    );
}
