pub use crate::contract::*;
pub use crate::ext_query::*;
pub use crate::state::*;
pub use cluster_math::*;
pub use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
pub use cosmwasm_std::*;
pub use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20::{Cw20QueryMsg, TokenInfoResponse};
use nebula_protocol::{
    cluster::InitMsg,
    cluster_factory::{ConfigResponse, QueryMsg as FactoryQueryMsg},
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    penalty::{MintResponse, PenaltyParams, QueryMsg as PenaltyQueryMsg, RedeemResponse},
};
use std::collections::HashMap;
pub use std::str::FromStr;
use terra_cosmwasm::*;
use terraswap::asset::AssetInfo;

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

pub struct WasmMockQuerier {
    pub base: MockQuerier<TerraQueryWrapper>,
    pub token_querier: TokenQuerier,     // token balances
    pub balance_querier: BalanceQuerier, // native balances
    pub oracle_querier: OracleQuerier,   // token registered prices
    pub penalty_querier: PenaltyQuerier, // penalty querier
    pub canonical_length: usize,
}

impl Querier for WasmMockQuerier {
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

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper {
                route: _,
                query_data: _,
            }) => panic!("Tried to access Terra query -- not implemented"),
            QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
                // Do for native
                let denom_data = match self.balance_querier.balances.get(denom) {
                    Some(v) => v,
                    None => {
                        return Err(SystemError::InvalidRequest {
                            error: format!("Denom not found in balances"),
                            request: Binary(vec![]),
                        })
                    }
                };
                let balance = match denom_data.get(&address) {
                    Some(v) => v,
                    None => &Uint128(0),
                };
                Ok(to_binary(&BalanceResponse {
                    amount: coin(balance.u128(), denom),
                }))
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg) {
                    Ok(OracleQueryMsg::Price {
                        base_asset,
                        quote_asset,
                    }) => match self.oracle_querier.assets.get(&base_asset) {
                        Some(base_price) => match self.oracle_querier.assets.get(&quote_asset) {
                            Some(quote_price) => Ok(to_binary(&PriceResponse {
                                rate: decimal_division(*base_price, *quote_price),
                                last_updated_base: u64::MAX,
                                last_updated_quote: u64::MAX,
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
                    _ => match from_binary(&msg) {
                        Ok(Cw20QueryMsg::Balance { address }) => {
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
                        Ok(Cw20QueryMsg::TokenInfo {}) => {
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
                        _ => match from_binary(&msg) {
                            Ok(FactoryQueryMsg::Config {}) => Ok(to_binary(&ConfigResponse {
                                owner: HumanAddr::from("owner"),
                                nebula_token: HumanAddr::from("nebula"),
                                staking_contract: HumanAddr::from("staking"),
                                commission_collector: HumanAddr::from("collector"),
                                protocol_fee_rate: "0.03".to_string(),
                                oracle_contract: HumanAddr::from("oracle"),
                                terraswap_factory: HumanAddr::from("terraswap_factory"),
                                token_code_id: 1,
                                cluster_code_id: 2,
                                base_denom: "uusd".to_string(),
                                genesis_time: 1000,
                                distribution_schedule: vec![],
                            })),
                            _ => match from_binary(&msg) {
                                Ok(PenaltyQueryMsg::Mint {
                                    block_height: _,
                                    cluster_token_supply: _,
                                    inventory: _,
                                    mint_asset_amounts: _,
                                    asset_prices: _,
                                    target_weights: _,
                                }) => {
                                    let response = MintResponse {
                                        mint_tokens: self.penalty_querier.mint_tokens,
                                        penalty: Uint128(1234),
                                        log: vec![log("penalty", 1234)],
                                    };
                                    Ok(to_binary(&response))
                                }
                                Ok(PenaltyQueryMsg::Redeem {
                                    block_height: _,
                                    cluster_token_supply: _,
                                    inventory: _,
                                    max_tokens: _,
                                    redeem_asset_amounts: _,
                                    asset_prices: _,
                                    target_weights: _,
                                }) => {
                                    let response = RedeemResponse {
                                        redeem_assets: self.penalty_querier.redeem_assets.clone(),
                                        token_cost: self.penalty_querier.token_cost,
                                        penalty: Uint128(1234),
                                        log: vec![log("penalty", 1234)],
                                    };
                                    Ok(to_binary(&response))
                                }
                                _ => {
                                    panic!("ExtQueryMsg type not implemented");
                                }
                            },
                        },
                    },
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

pub fn token_data<T, U>(
    name: &str,
    symbol: &str,
    decimals: u8,
    total_supply: u128,
    balances: T,
) -> TokenData
where
    T: IntoIterator<Item = (U, u128)>,
    U: Into<HumanAddr>,
{
    let mut balances_map: HashMap<HumanAddr, Uint128> = HashMap::new();
    for (account_addr, balance) in balances.into_iter() {
        balances_map.insert(account_addr.into(), Uint128(balance));
    }

    TokenData {
        info: TokenInfoResponse {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply: Uint128(total_supply),
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

#[derive(Default)]
pub struct PenaltyQuerier {
    pub mint_tokens: Uint128,
    pub token_cost: Uint128,
    pub redeem_assets: Vec<Uint128>,
}

impl PenaltyQuerier {
    pub fn new() -> Self {
        PenaltyQuerier {
            mint_tokens: Uint128::zero(),
            token_cost: Uint128::zero(),
            redeem_assets: vec![],
        }
    }
}

#[derive(Default)]
pub struct BalanceQuerier {
    // this lets us iterate over all pairs that match the first string

    // balances: denom -> account address -> amount
    pub balances: HashMap<String, HashMap<HumanAddr, Uint128>>,
}

impl BalanceQuerier {
    pub fn new() -> Self {
        BalanceQuerier {
            balances: HashMap::new(),
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

impl WasmMockQuerier {
    pub fn new<A: Api>(
        base: MockQuerier<TerraQueryWrapper>,
        _api: A,
        canonical_length: usize,
    ) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            balance_querier: BalanceQuerier::default(),
            penalty_querier: PenaltyQuerier::default(),
            canonical_length,
        }
    }

    // configure the mint whitelist mock querier
    pub fn reset_token_querier(&mut self) -> &mut Self {
        self.token_querier = TokenQuerier::new();
        self
    }

    pub fn set_token<T>(&mut self, token_address: T, data: TokenData) -> &mut Self
    where
        T: Into<HumanAddr>,
    {
        self.token_querier.tokens.insert(token_address.into(), data);
        self
    }

    pub fn set_denom<T>(&mut self, denom: T, balances: HashMap<HumanAddr, Uint128>) -> &mut Self
    where
        T: Into<String>,
    {
        self.balance_querier.balances.insert(denom.into(), balances);
        self
    }

    pub fn set_token_supply<T>(&mut self, token_address: T, supply: u128) -> &mut Self
    where
        T: Into<HumanAddr>,
    {
        if let Some(token) = self.token_querier.tokens.get_mut(&token_address.into()) {
            token.info.total_supply = Uint128(supply);
        }
        self
    }

    pub fn set_token_balance<T, U>(
        &mut self,
        token_address: T,
        account_address: U,
        balance: u128,
    ) -> &mut Self
    where
        T: Into<HumanAddr>,
        U: Into<HumanAddr>,
    {
        if let Some(token) = self.token_querier.tokens.get_mut(&token_address.into()) {
            token
                .balances
                .insert(account_address.into(), Uint128(balance));
        }
        self
    }

    pub fn set_denom_balance<T, U>(
        &mut self,
        denom: T,
        account_address: U,
        balance: u128,
    ) -> &mut Self
    where
        T: Into<String>,
        U: Into<HumanAddr>,
    {
        if let Some(denom) = self.balance_querier.balances.get_mut(&denom.into()) {
            denom.insert(account_address.into(), Uint128(balance));
        }
        self
    }

    // configure the oracle price mock querier
    pub fn reset_oracle_querier(&mut self) -> &mut Self {
        self.oracle_querier = OracleQuerier::new();
        self
    }

    pub fn set_oracle_price(&mut self, asset_address: String, price: Decimal) -> &mut Self {
        self.oracle_querier.assets.insert(asset_address, price);
        self
    }

    pub fn set_oracle_prices<T, U>(&mut self, price_data: T) -> &mut Self
    where
        T: IntoIterator<Item = (U, Decimal)>,
        U: ToString,
    {
        for (asset, price) in price_data.into_iter() {
            self.set_oracle_price(asset.to_string(), price);
        }
        self
    }

    pub fn set_mint_amount(&mut self, mint_tokens: Uint128) -> &mut Self {
        self.penalty_querier.mint_tokens = mint_tokens;
        self
    }

    pub fn set_redeem_amount(
        &mut self,
        token_cost: Uint128,
        redeem_assets: Vec<Uint128>,
    ) -> &mut Self {
        self.penalty_querier.token_cost = token_cost;
        self.penalty_querier.redeem_assets = redeem_assets;
        self
    }
}

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Extern<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = HumanAddr::from(MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
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
        "test_cluster"
    }

    pub fn description() -> &'static str {
        "description"
    }

    pub fn owner() -> HumanAddr {
        h("owner")
    }
    pub fn cluster_token() -> HumanAddr {
        h("cluster")
    }
    pub fn factory() -> HumanAddr {
        h("factory")
    }
    pub fn pricing_oracle() -> HumanAddr {
        h("pricing_oracle")
    }
    pub fn composition_oracle() -> HumanAddr {
        h("composition_oracle")
    }
    pub fn assets_stage() -> Vec<AssetInfo> {
        vec![
            AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
        ]
    }
    pub fn target() -> Vec<u32> {
        vec![20, 10, 65, 5]
    }
    pub fn assets_native_stage() -> Vec<AssetInfo> {
        vec![
            AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            AssetInfo::Token {
                contract_addr: h("mNFLX"),
            },
            AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
        ]
    }

    pub fn target_stage() -> Vec<u32> {
        vec![20, 20, 20, 20]
    }

    pub fn target_native_stage() -> Vec<u32> {
        vec![20, 20, 20, 20, 20]
    }

    pub fn penalty() -> HumanAddr {
        h("penalty")
    }
}

pub fn mock_init() -> (Extern<MockStorage, MockApi, WasmMockQuerier>, InitResponse) {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        assets: consts::assets_stage(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: consts::target_stage(),
        pricing_oracle: consts::pricing_oracle(),
        composition_oracle: consts::composition_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
        init_hook: None,
    };

    let env = mock_env(consts::pricing_oracle().as_str(), &[]);
    let res = init(&mut deps, env.clone(), msg).unwrap();
    (deps, res)
}

pub fn mock_init_native_stage() -> (Extern<MockStorage, MockApi, WasmMockQuerier>, InitResponse) {
    let mut deps = mock_dependencies(20, &[]);
    let msg = InitMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        assets: consts::assets_native_stage(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: consts::target_native_stage(),
        pricing_oracle: consts::pricing_oracle(),
        composition_oracle: consts::composition_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
        init_hook: None,
    };

    let env = mock_env(consts::pricing_oracle().as_str(), &[]);
    let res = init(&mut deps, env.clone(), msg).unwrap();
    (deps, res)
}

/// sets up mock queriers with basic setup
pub fn mock_querier_setup(deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    deps.querier
        .reset_token_querier()
        .set_token(
            consts::cluster_token(),
            token_data::<Vec<(&str, u128)>, &str>(
                "Cluster Token",
                "CLUSTER",
                6,
                1_000_000_000,
                vec![],
            ),
        )
        .set_token(
            "mAAPL",
            token_data(
                "Mirrored Apple",
                "mAAPL",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        )
        .set_token(
            "mGOOG",
            token_data(
                "Mirrored Google",
                "mGOOG",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        )
        .set_token(
            "mMSFT",
            token_data(
                "Mirrored Microsoft",
                "mMSFT",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        )
        .set_token(
            "mNFLX",
            token_data(
                "Mirrored Netflix",
                "mNFLX",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        );

    deps.querier.reset_oracle_querier().set_oracle_prices(vec![
        ("uusd", Decimal::one()),
        ("mAAPL", Decimal::from_str("1.0").unwrap()),
        ("mGOOG", Decimal::from_str("1.0").unwrap()),
        ("mMSFT", Decimal::from_str("1.0").unwrap()),
        ("mNFLX", Decimal::from_str("1.0").unwrap()),
    ]);
}

/// sets up mock queriers with basic setup
pub fn mock_querier_setup_stage_native(deps: &mut Extern<MockStorage, MockApi, WasmMockQuerier>) {
    deps.querier
        .reset_token_querier()
        .set_token(
            consts::cluster_token(),
            token_data::<Vec<(&str, u128)>, &str>(
                "Cluster Token",
                "CLUSTER",
                6,
                1_000_000_000,
                vec![],
            ),
        )
        .set_token(
            "wBTC",
            token_data(
                "Wrapped BTC",
                "wBTC",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        )
        .set_denom("uluna", HashMap::new());

    deps.querier.reset_oracle_querier().set_oracle_prices(vec![
        ("uusd", Decimal::one()),
        ("wBTC", Decimal::from_str("1.0").unwrap()),
        ("uluna", Decimal::from_str("1.0").unwrap()),
    ]);
}
