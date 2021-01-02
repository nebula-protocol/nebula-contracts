pub use crate::contract::*;
pub use crate::ext_query::*;
pub use crate::msg::*;
pub use crate::penalty::*;
pub use crate::state::*;
pub use basket_math::*;
pub use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
pub use cosmwasm_std::*;
pub use cw20::BalanceResponse as Cw20BalanceResponse;
use std::collections::HashMap;
pub use std::str::FromStr;
use terra_cosmwasm::*;
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

pub struct CustomMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    token_querier: TokenQuerier,   // token balances
    oracle_querier: OracleQuerier, // token registered prices
    canonical_length: usize,
}

impl Querier for CustomMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

const DECIMAL_FRACTIONAL: Uint128 = Uint128(1_000_000_000u128);
pub fn decimal_division(a: Decimal, b: Decimal) -> Decimal {
    Decimal::from_ratio(DECIMAL_FRACTIONAL * a, b * DECIMAL_FRACTIONAL)
}

impl CustomMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                panic!("Tried to access Terra query -- not implemented")
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg).unwrap() {
                    ExtQueryMsg::Price {
                        base_asset,
                        quote_asset,
                    } => match self.oracle_querier.price.get(&base_asset) {
                        Some(base_price) => match self.oracle_querier.price.get(&quote_asset) {
                            Some(quote_price) => Ok(to_binary(&PriceResponse {
                                rate: decimal_division(*base_price, *quote_price),
                                last_updated_base: 1000u64,
                                last_updated_quote: 1000u64,
                            })),
                            None => Err(SystemError::InvalidRequest {
                                error: "No oracle price exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        },
                        None => Err(SystemError::InvalidRequest {
                            error: "No oracle price exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    },
                    ExtQueryMsg::Balance { address } => {
                        let balances = match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
                            None => {
                                return Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No balance info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: msg.as_slice().into(),
                                })
                            }
                        };
                        let balance = match balances.get(&address) {
                            Some(v) => v,
                            None => {
                                return Err(SystemError::InvalidRequest {
                                    error: "Balance not found".to_string(),
                                    request: msg.as_slice().into(),
                                })
                            }
                        };
                        Ok(to_binary(&Cw20BalanceResponse { balance: *balance }))
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])],
) -> HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> {
    let mut balances_map: HashMap<HumanAddr, HashMap<HumanAddr, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<HumanAddr, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(HumanAddr::from(addr), **balance);
        }

        balances_map.insert(HumanAddr::from(contract_addr), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct OracleQuerier {
    // this lets us iterate over all pairs that match the first string
    price: HashMap<String, Decimal>,
}

impl OracleQuerier {
    pub fn new(price: &[(&String, &Decimal)]) -> Self {
        OracleQuerier {
            price: price_to_map(price),
        }
    }
}

pub(crate) fn price_to_map(price: &[(&String, &Decimal)]) -> HashMap<String, Decimal> {
    let mut price_map: HashMap<String, Decimal> = HashMap::new();
    for (base_quote, oracle_price) in price.iter() {
        price_map.insert((*base_quote).clone(), **oracle_price);
    }

    price_map
}

impl CustomMockQuerier {
    pub fn new<A: Api>(
        base: MockQuerier<TerraQueryWrapper>,
        _api: A,
        canonical_length: usize,
    ) -> Self {
        CustomMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            canonical_length,
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&HumanAddr, &[(&HumanAddr, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the oracle price mock querier
    pub fn with_oracle_prices(&mut self, oracle_prices: &[(&String, &Decimal)]) {
        self.oracle_querier = OracleQuerier::new(oracle_prices);
    }
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, CustomMockQuerier> {
    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let custom_querier: CustomMockQuerier = CustomMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        MockApi::new(canonical_length),
        canonical_length,
    );

    Extern {
        storage: MockStorage::default(),
        api: MockApi::new(canonical_length),
        querier: custom_querier,
    }
}

pub mod consts {

    use super::*;

    pub fn name() -> &'static str {
        "test_basket"
    }

    pub fn owner() -> HumanAddr {
        h("owner0000")
    }
    pub fn basket_token() -> HumanAddr {
        h("token0000")
    }
    pub fn oracle() -> HumanAddr {
        h("oracle0000")
    }
    pub fn assets() -> Vec<HumanAddr> {
        vec![h("mAAPL"), h("mGOOG"), h("mMSFT"), h("mNFLX")]
    }
    pub fn target() -> Vec<u32> {
        vec![1, 1, 2, 1]
    }
    pub fn penalty_params() -> PenaltyParams {
        PenaltyParams {
            a_pos: FPDecimal::from_str("1.0").unwrap(),
            s_pos: FPDecimal::from_str("1.0").unwrap(),
            a_neg: FPDecimal::from_str("0.005").unwrap(),
            s_neg: FPDecimal::from_str("0.5").unwrap(),
        }
    }
}

pub fn mock_init() -> (
    Extern<MockStorage, MockApi, CustomMockQuerier>,
    InitResponse,
) {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        name: consts::name().to_string(),
        assets: consts::assets(),
        owner: consts::owner(),
        basket_token: consts::basket_token(),
        target: consts::target(),
        oracle: consts::oracle(),
        penalty_params: consts::penalty_params(),
    };

    let env = mock_env(consts::oracle().as_str(), &[]);
    let res = init(&mut deps, env.clone(), msg).unwrap();
    (deps, res)
}
