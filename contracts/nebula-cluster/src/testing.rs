use super::*;
pub use crate::contract::*;
pub use crate::ext_query::*;
pub use crate::state::*;
pub use cluster_math::*;
pub use cosmwasm_std::testing::{
    mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
};
pub use cosmwasm_std::*;
pub use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
use nebula_protocol::penalty::ExecuteMsg as PenaltyExecuteMsg;
use nebula_protocol::{
    cluster::{ExecuteMsg, InstantiateMsg, QueryMsg as ClusterQueryMsg, TargetResponse},
    cluster_factory::ConfigResponse as FactoryConfigResponse,
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    penalty::{MintResponse, QueryMsg as PenaltyQueryMsg, RedeemResponse},
};
use pretty_assertions::assert_eq;
use std::collections::HashMap;
pub use std::str::FromStr;
use terra_cosmwasm::*;
use terraswap::asset::{Asset, AssetInfo};

/// Convenience function for creating inline String
pub fn h(s: &str) -> String {
    s.to_string()
}

#[macro_export]
macro_rules! q {
    ($deps:expr, $val_type:ty, $env:expr, $msg: expr) => {{
        let res = query($deps, $env, $msg).unwrap();
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
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.execute_query(&request)
    }
}

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);
pub fn decimal_division(a: Decimal, b: Decimal) -> Decimal {
    Decimal::from_ratio(DECIMAL_FRACTIONAL * a, b * DECIMAL_FRACTIONAL)
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
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
                        return SystemResult::Err(SystemError::InvalidRequest {
                            error: format!("Denom not found in balances"),
                            request: Binary(vec![]),
                        })
                    }
                };
                let balance = match denom_data.get(address) {
                    Some(v) => v.clone(),
                    None => Uint128::zero(),
                };
                SystemResult::Ok(ContractResult::from(to_binary(&BalanceResponse {
                    amount: coin(balance.u128(), denom),
                })))
            }
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(&msg) {
                    Ok(OracleQueryMsg::Price {
                        base_asset,
                        quote_asset,
                    }) => match self.oracle_querier.assets.get(&base_asset) {
                        Some(base_price) => match self.oracle_querier.assets.get(&quote_asset) {
                            Some(quote_price) => {
                                SystemResult::Ok(ContractResult::from(to_binary(&PriceResponse {
                                    rate: decimal_division(*base_price, *quote_price),
                                    last_updated_base: u64::MAX,
                                    last_updated_quote: u64::MAX,
                                })))
                            }
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No oracle price exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        },
                        None => SystemResult::Err(SystemError::InvalidRequest {
                            error: "No oracle price exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    },
                    _ => match from_binary(&msg) {
                        Ok(Cw20QueryMsg::Balance { address }) => {
                            let token_data = match self.token_querier.tokens.get(contract_addr) {
                                Some(v) => v,
                                None => {
                                    return SystemResult::Err(SystemError::InvalidRequest {
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
                                    return SystemResult::Err(SystemError::InvalidRequest {
                                        error: "Balance not found".to_string(),
                                        request: msg.as_slice().into(),
                                    })
                                }
                            };
                            SystemResult::Ok(ContractResult::from(to_binary(
                                &Cw20BalanceResponse { balance: *balance },
                            )))
                        }
                        Ok(Cw20QueryMsg::TokenInfo {}) => {
                            let token_data = match self.token_querier.tokens.get(contract_addr) {
                                Some(v) => v,
                                None => {
                                    return SystemResult::Err(SystemError::InvalidRequest {
                                        error: format!(
                                            "No token info exists for the contract {}",
                                            contract_addr
                                        ),
                                        request: msg.as_slice().into(),
                                    })
                                }
                            };
                            SystemResult::Ok(ContractResult::from(to_binary(&token_data.info)))
                        }
                        _ => match from_binary(&msg) {
                            Ok(ClusterQueryMsg::Config {}) => {
                                let config = consts::factory_config();
                                SystemResult::Ok(ContractResult::from(to_binary(&config)))
                            }
                            _ => match from_binary(&msg) {
                                Ok(PenaltyQueryMsg::PenaltyQueryCreate {
                                    block_height: _,
                                    cluster_token_supply: _,
                                    inventory: _,
                                    mint_asset_amounts: _,
                                    asset_prices: _,
                                    target_weights: _,
                                }) => {
                                    let response = consts::mint_response();
                                    SystemResult::Ok(ContractResult::from(to_binary(&response)))
                                }
                                Ok(PenaltyQueryMsg::PenaltyQueryRedeem {
                                    block_height: _,
                                    cluster_token_supply: _,
                                    inventory: _,
                                    max_tokens: _,
                                    asset_prices: _,
                                    target_weights: _,
                                    redeem_asset_amounts: _,
                                }) => {
                                    let response = RedeemResponse {
                                        redeem_assets: vec![
                                            Uint128::new(99),
                                            Uint128::new(98),
                                            Uint128::new(97),
                                            Uint128::new(96),
                                        ],
                                        penalty: Uint128::new(1234),
                                        token_cost: Uint128::new(1234),
                                        attributes: vec![attr("penalty", "1234")],
                                    };
                                    SystemResult::Ok(ContractResult::from(to_binary(&response)))
                                }
                                _ => {
                                    panic!("QueryMsg type not implemented");
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
    balances: HashMap<String, Uint128>,
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
    U: Into<String>,
{
    let mut balances_map: HashMap<String, Uint128> = HashMap::new();
    for (account_addr, balance) in balances.into_iter() {
        balances_map.insert(account_addr.into(), Uint128::new(balance));
    }

    TokenData {
        info: TokenInfoResponse {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply: Uint128::new(total_supply),
        },
        balances: balances_map,
    }
}

#[derive(Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    pub tokens: HashMap<String, TokenData>,
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
    pub balances: HashMap<String, HashMap<String, Uint128>>,
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
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            balance_querier: BalanceQuerier::default(),
            penalty_querier: PenaltyQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn reset_token_querier(&mut self) -> &mut Self {
        self.token_querier = TokenQuerier::new();
        self
    }

    pub fn set_token<T>(&mut self, token_address: T, data: TokenData) -> &mut Self
    where
        T: Into<String>,
    {
        self.token_querier.tokens.insert(token_address.into(), data);
        self
    }

    pub fn set_denom<T>(&mut self, denom: T, balances: HashMap<String, Uint128>) -> &mut Self
    where
        T: Into<String>,
    {
        self.balance_querier.balances.insert(denom.into(), balances);
        self
    }

    pub fn set_token_supply<T>(&mut self, token_address: T, supply: u128) -> &mut Self
    where
        T: Into<String>,
    {
        if let Some(token) = self.token_querier.tokens.get_mut(&token_address.into()) {
            token.info.total_supply = Uint128::new(supply);
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
        T: Into<String>,
        U: Into<String>,
    {
        if let Some(token) = self.token_querier.tokens.get_mut(&token_address.into()) {
            token
                .balances
                .insert(account_address.into(), Uint128::new(balance));
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
        U: Into<String>,
    {
        if let Some(denom) = self.balance_querier.balances.get_mut(&denom.into()) {
            denom.insert(account_address.into(), Uint128::new(balance));
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
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = MOCK_CONTRACT_ADDR.to_string();
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(&contract_addr, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
    }
}

pub mod consts {

    use terraswap::asset::Asset;

    use super::*;

    pub fn name() -> &'static str {
        "test_cluster"
    }

    pub fn description() -> &'static str {
        "description"
    }

    pub fn owner() -> String {
        h("owner")
    }
    pub fn cluster_token() -> String {
        h("cluster")
    }
    pub fn factory() -> String {
        h("factory")
    }
    pub fn pricing_oracle() -> String {
        h("pricing_oracle")
    }
    pub fn target_oracle() -> String {
        h("target_oracle")
    }
    pub fn target_assets_stage() -> Vec<Asset> {
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128::new(20),
            },
        ]
    }
    pub fn target() -> Vec<u32> {
        vec![20, 10, 65, 5]
    }
    pub fn target_assets_native_stage() -> Vec<Asset> {
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ukrw".to_string(),
                },
                amount: Uint128::new(20),
            },
        ]
    }

    pub fn target_stage() -> Vec<Uint128> {
        vec![
            Uint128::new(20),
            Uint128::new(20),
            Uint128::new(20),
            Uint128::new(20),
        ]
    }

    pub fn target_native_stage() -> Vec<Uint128> {
        vec![
            Uint128::new(20),
            Uint128::new(20),
            Uint128::new(20),
            Uint128::new(20),
            Uint128::new(20),
        ]
    }

    pub fn penalty() -> String {
        h("penalty")
    }

    pub fn factory_config() -> FactoryConfigResponse {
        FactoryConfigResponse {
            owner: h("gov"),
            nebula_token: h("neb"),
            staking_contract: "staking".to_string(),
            commission_collector: h("collector"),
            protocol_fee_rate: "0.01".to_string(),
            terraswap_factory: h("ts_factory"),
            token_code_id: 1,
            cluster_code_id: 1,
            base_denom: "uusd".to_string(),
            genesis_time: 1,
            distribution_schedule: vec![(1, 2, Uint128::from(123u128))],
        }
    }

    pub fn mint_response() -> MintResponse {
        MintResponse {
            mint_tokens: Uint128::new(99),
            penalty: Uint128::new(1234),
            attributes: vec![attr("penalty", "1234")],
        }
    }

    pub fn asset_amounts() -> Vec<Asset> {
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128::new(125_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::zero(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128::new(149_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128::new(50_090_272),
            },
        ]
    }
}

pub fn mock_init() -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Response) {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: consts::target_assets_stage(),
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };

    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    (deps, res)
}

pub fn mock_init_native_stage() -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Response) {
    let mut deps = mock_dependencies(&[]);
    let msg = InstantiateMsg {
        name: consts::name().to_string(),
        description: consts::description().to_string(),
        owner: consts::owner(),
        cluster_token: Some(consts::cluster_token()),
        target: consts::target_assets_native_stage(),
        pricing_oracle: consts::pricing_oracle(),
        target_oracle: consts::target_oracle(),
        penalty: consts::penalty(),
        factory: consts::factory(),
    };

    let info = mock_info(consts::pricing_oracle().as_str(), &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    (deps, res)
}

/// sets up mock queriers with basic setup
pub fn mock_querier_setup(
    mut deps: OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    deps.querier
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

    deps.querier.set_oracle_prices(vec![
        ("uusd", Decimal::one()),
        ("mAAPL", Decimal::from_str("1.0").unwrap()),
        ("mGOOG", Decimal::from_str("1.0").unwrap()),
        ("mMSFT", Decimal::from_str("1.0").unwrap()),
        ("mNFLX", Decimal::from_str("1.0").unwrap()),
    ]);

    deps
}

/// sets up mock queriers with basic setup
pub fn mock_querier_setup_stage_native(
    mut deps: OwnedDeps<MockStorage, MockApi, WasmMockQuerier>,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    deps.querier
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

    deps.querier.set_oracle_prices(vec![
        ("uusd", Decimal::one()),
        ("wBTC", Decimal::from_str("1.0").unwrap()),
        ("uluna", Decimal::from_str("1.0").unwrap()),
    ]);

    deps
}

#[test]
fn proper_initialization() {
    let (deps, init_res) = mock_init();
    assert_eq!(0, init_res.messages.len());

    // make sure target was saved
    let value = q!(
        deps.as_ref(),
        TargetResponse,
        mock_env(),
        ClusterQueryMsg::Target {}
    );
    assert_eq!(
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mAAPL"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mGOOG"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mMSFT"),
                },
                amount: Uint128::new(20)
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: h("mNFLX"),
                },
                amount: Uint128::new(20)
            },
        ],
        value.target
    );
}

#[test]
fn mint() {
    let (mut deps, _) = mock_init();
    deps = mock_querier_setup(deps);
    // Asset :: UST Price :: Balance (µ)     (+ proposed   ) :: %
    // ---
    // mAAPL ::  135.18   ::  7_290_053_159  (+ 125_000_000) :: 0.20367359382 -> 0.20391741720
    // mGOOG :: 1780.03   ::    319_710_128                  :: 0.11761841035 -> 0.11577407690
    // mMSFT ::  222.42   :: 14_219_281_228  (+ 149_000_000) :: 0.65364669475 -> 0.65013907200
    // mNFLX ::  540.82   ::    224_212_221  (+  50_090_272) :: 0.02506130106 -> 0.03016943389

    // The set token balance should include the amount we would also like to stage
    deps.querier
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let asset_amounts = consts::asset_amounts();

    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let mint_msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let addr = "addr0000";
    let info = mock_info(addr, &[]);
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info, mint_msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "mint"),
            attr("sender", "addr0000"),
            attr("mint_to_sender", "98"),
            attr("penalty", "1234"),
            attr("fee_amt", "1"),
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mAAPL"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(125_000_000),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mGOOG"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::zero(),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mMSFT"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(149_000_000),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mNFLX"),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    recipient: MOCK_CONTRACT_ADDR.to_string(),
                    amount: Uint128::new(50_090_272),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyExecuteMsg::PenaltyCreate {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128::new(1_000_000_000),
                    inventory: vec![
                        Uint128::new(7_290_053_159u128),
                        Uint128::new(319_710_128u128),
                        Uint128::new(14_219_281_228u128),
                        Uint128::new(224_212_221u128)
                    ],
                    mint_asset_amounts: vec![
                        Uint128::new(125_000_000),
                        Uint128::zero(),
                        Uint128::new(149_000_000),
                        Uint128::new(50_090_272),
                    ],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string()
                    ],
                    target_weights: vec![
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128)
                    ],
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: Uint128::new(1u128),
                    recipient: h("collector"),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    amount: Uint128::new(98),
                    recipient: "addr0000".to_string(),
                })
                .unwrap(),
                funds: vec![],
            }))
        ]
    );
}

#[test]
fn burn() {
    let (mut deps, _init_res) = mock_init();
    deps = mock_querier_setup(deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000)
        .set_token_balance("mAAPL", MOCK_CONTRACT_ADDR, 7_290_053_159)
        .set_token_balance("mGOOG", MOCK_CONTRACT_ADDR, 319_710_128)
        .set_token_balance("mMSFT", MOCK_CONTRACT_ADDR, 14_219_281_228)
        .set_token_balance("mNFLX", MOCK_CONTRACT_ADDR, 224_212_221)
        .set_oracle_prices(vec![
            ("mAAPL", Decimal::from_str("135.18").unwrap()),
            ("mGOOG", Decimal::from_str("1780.03").unwrap()),
            ("mMSFT", Decimal::from_str("222.42").unwrap()),
            ("mNFLX", Decimal::from_str("540.82").unwrap()),
        ]);

    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts: None,
    };
    let info = mock_info("addr0000", &[]);
    let env = mock_env();
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "receive:burn"),
            attr("sender", "addr0000"),
            attr("burn_amount", "1234"),
            attr("token_cost", "1247"),
            attr("kept_as_fee", "13"),
            attr("asset_amounts", "[]"),
            attr("redeem_totals", "[99, 98, 97, 96]"),
            attr("penalty", "1234")
        ]
    );

    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mAAPL"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(99u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mGOOG"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(98u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mMSFT"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(97u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: h("mNFLX"),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "addr0000".to_string(),
                    amount: Uint128::new(96u128)
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: "addr0000".to_string(),
                    amount: Uint128::new(13u128),
                    recipient: h("collector"),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::cluster_token(),
                msg: to_binary(&Cw20ExecuteMsg::BurnFrom {
                    owner: "addr0000".to_string(),
                    amount: Uint128::new(1234u128),
                })
                .unwrap(),
                funds: vec![],
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: consts::penalty(),
                msg: to_binary(&PenaltyQueryMsg::PenaltyQueryRedeem {
                    block_height: env.block.height,
                    cluster_token_supply: Uint128::new(100_000_000u128),
                    inventory: vec![
                        Uint128::new(7_290_053_159u128),
                        Uint128::new(319_710_128u128),
                        Uint128::new(14_219_281_228u128),
                        Uint128::new(224_212_221u128)
                    ],
                    max_tokens: Uint128::new(20_000_000u128),
                    redeem_asset_amounts: vec![],
                    asset_prices: vec![
                        "135.18".to_string(),
                        "1780.03".to_string(),
                        "222.42".to_string(),
                        "540.82".to_string()
                    ],
                    target_weights: vec![
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128),
                        Uint128::new(20u128)
                    ],
                })
                .unwrap(),
                funds: vec![],
            })),
        ]
    );
}

#[test]
fn update_target() {
    let (mut deps, _init_res) = mock_init();
    deps = mock_querier_setup(deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    let new_target: Vec<Asset> = vec![
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mAAPL"),
            },
            amount: Uint128::new(10),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGOOG"),
            },
            amount: Uint128::new(5),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mMSFT"),
            },
            amount: Uint128::new(35),
        },
        Asset {
            info: AssetInfo::Token {
                contract_addr: h("mGME"),
            },
            amount: Uint128::new(50),
        },
    ];
    let msg = ExecuteMsg::UpdateTarget { target: new_target };

    let info = mock_info(consts::owner().as_str(), &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "reset_target"),
            attr("prev_assets", "[mAAPL, mGOOG, mMSFT, mNFLX]"),
            attr("prev_targets", "[20, 20, 20, 20]"),
            attr("updated_assets", "[mAAPL, mGOOG, mMSFT, mGME, mNFLX]"),
            attr("updated_targets", "[10, 5, 35, 50, 0]"),
        ]
    );

    assert_eq!(res.messages, vec![]);
}

#[test]
fn decommission_cluster() {
    let (mut deps, _init_res) = mock_init();
    deps = mock_querier_setup(deps);

    deps.querier
        .set_token_supply(consts::cluster_token(), 100_000_000)
        .set_token_balance(consts::cluster_token(), "addr0000", 20_000_000);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, true);

    let msg = ExecuteMsg::Decommission {};

    let info = mock_info("owner0001", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();

    match res {
        StdError::GenericErr { msg, .. } => assert_eq!(msg, "unauthorized"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    let info = mock_info(consts::factory().as_str(), &[]);

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

    assert_eq!(res.attributes, vec![attr("action", "decommission_asset")]);

    let config = read_config(&deps.storage).unwrap();
    assert_eq!(config.active, false);

    assert_eq!(res.messages, vec![]);

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();

    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot decommission an already decommissioned cluster")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let asset_amounts = consts::asset_amounts();
    deps.querier.set_mint_amount(Uint128::from(1_000_000u128));

    let msg = ExecuteMsg::RebalanceCreate {
        asset_amounts: asset_amounts.clone(),
        min_tokens: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(msg, "Cannot call mint on a decommissioned cluster")
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts: Some(asset_amounts),
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    match res {
        StdError::GenericErr { msg, .. } => {
            assert_eq!(
                msg,
                "Cannot call non pro-rata redeem on a decommissioned cluster"
            )
        }
        _ => panic!("DO NOT ENTER HERE"),
    }

    let msg = ExecuteMsg::RebalanceRedeem {
        max_tokens: Uint128::new(20_000_000),
        asset_amounts: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "receive:burn"),
            attr("sender", "factory"),
            attr("burn_amount", "1234"),
            attr("token_cost", "1247"),
            attr("kept_as_fee", "13"),
            attr("asset_amounts", "[]"),
            attr("redeem_totals", "[99, 98, 97, 96]"),
            attr("penalty", "1234")
        ]
    );
}
