pub mod prelude {

    pub use crate::contract::*;
    pub use crate::msg::*;
    pub use crate::penalty::*;
    pub use crate::state::*;
    pub use basket_math::*;
    pub use cosmwasm_std::testing::{mock_dependencies, mock_env};
    pub use cosmwasm_std::*;
    pub use std::str::FromStr;
    use testing::{MockApi, MockQuerier, MockStorage};

    /// Convenience function for creating inline HumanAddr
    pub fn h(s: &str) -> HumanAddr {
        HumanAddr(s.to_string())
    }

    #[macro_export]
    macro_rules! q {
        ($deps:expr, $val_type:ty, $msg: expr) => {{
            let res = query($deps, $msg).unwrap();
            let val: $val_type = from_binary(&res).unwrap();
            val
        }};
    }

    pub fn init_contract() -> (Extern<MockStorage, MockApi, MockQuerier>, InitResponse) {
        let mut deps = mock_dependencies(20, &[]);
        let name = "test-basket";
        let owner = h("owner0000");
        let basket_token = h("token0000");
        let oracle = h("oracle0000");

        let msg = InitMsg {
            name: name.to_string(),
            assets: vec![h("mAAPL"), h("mGOOG"), h("mMSFT"), h("mNFLX")],
            owner: owner.clone(),
            basket_token: basket_token.clone(),
            target: vec![1, 1, 2, 1],
            oracle: oracle.clone(),
            penalty_params: PenaltyParams {
                a_pos: FPDecimal::from_str("1.0").unwrap(),
                s_pos: FPDecimal::from_str("1.0").unwrap(),
                a_neg: FPDecimal::from_str("0.005").unwrap(),
                s_neg: FPDecimal::from_str("0.5").unwrap(),
            },
        };

        let env = mock_env(owner.as_str(), &[]);
        let res = init(&mut deps, env.clone(), msg).unwrap();
        (deps, res)
    }
}
