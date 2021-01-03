pub use crate::contract::*;
pub use crate::ext_query::*;
pub use crate::msg::*;
pub use crate::penalty::*;
pub use crate::state::*;
pub use basket_math::*;
pub use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
pub use cosmwasm_std::*;
pub use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20::TokenInfoResponse;
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
    pub base: MockQuerier<TerraQueryWrapper>,
    pub token_querier: TokenQuerier,   // token balances
    pub oracle_querier: OracleQuerier, // token registered prices
    pub canonical_length: usize,
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
                    } => match self.oracle_querier.assets.get(&base_asset) {
                        Some(base_price) => match self.oracle_querier.assets.get(&quote_asset) {
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
                        let token_data = match self.token_querier.tokens.get(contract_addr) {
                            Some(v) => v,
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
                        let balance = match token_data.balances.get(&address) {
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
                    ExtQueryMsg::TokenInfo {} => {
                        let token_data = match self.token_querier.tokens.get(contract_addr) {
                            Some(v) => v,
                            None => {
                                return Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "No token info exists for the contract {}",
                                        contract_addr
                                    ),
                                    request: msg.as_slice().into(),
                                })
                            }
                        };
                        Ok(to_binary(&token_data.info))
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

#[derive(Clone)]
pub struct TokenData {
    info: TokenInfoResponse,
    balances: HashMap<HumanAddr, Uint128>,
}

pub fn token_data(
    name: &str,
    symbol: &str,
    decimals: u8,
    total_supply: u128,
    balances: &[(&str, u128)],
) -> TokenData {
    let mut balances_map: HashMap<HumanAddr, Uint128> = HashMap::new();
    for &(account_addr, balance) in balances.iter() {
        balances_map.insert(account_addr.into(), Uint128(balance));
    }

    TokenData {
        info: TokenInfoResponse {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply: Uint128(total_supply.into()),
        },
        balances: balances_map,
    }
}

#[derive(Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    pub tokens: HashMap<HumanAddr, TokenData>,
}

impl TokenQuerier {
    pub fn new() -> Self {
        TokenQuerier {
            tokens: HashMap::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct OracleQuerier {
    // this lets us iterate over all pairs that match the first string
    pub assets: HashMap<String, Decimal>,
}

impl OracleQuerier {
    pub fn new() -> Self {
        OracleQuerier {
            assets: HashMap::new(),
        }
    }
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
    pub fn reset_token_querier(&mut self) -> &Self {
        self.token_querier = TokenQuerier::new();
        self
    }

    pub fn set_token<T>(&mut self, token_address: T, data: TokenData) -> &Self
    where
        T: Into<HumanAddr>,
    {
        self.token_querier.tokens.insert(token_address.into(), data);
        self
    }

    pub fn set_token_balance<T>(
        &mut self,
        token_address: T,
        account_address: T,
        balance: u128,
    ) -> &Self
    where
        T: Into<HumanAddr>,
    {
        if let Some(token) = self.token_querier.tokens.get_mut(&token_address.into()) {
            token
                .balances
                .insert(account_address.into(), Uint128(balance));
        }
        self
    }

    // configure the oracle price mock querier
    pub fn reset_oracle_querier(&mut self) -> &Self {
        self.oracle_querier = OracleQuerier::new();
        self
    }

    pub fn set_oracle_price(&mut self, asset_address: String, price: Decimal) -> &Self {
        self.oracle_querier.assets.insert(asset_address, price);
        self
    }

    pub fn set_oracle_prices<T, U>(&mut self, price_data: T) -> &Self
    where
        T: IntoIterator<Item = (U, Decimal)>,
        U: ToString,
    {
        for (asset, price) in price_data.into_iter() {
            self.set_oracle_price(asset.to_string(), price);
        }
        self
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
