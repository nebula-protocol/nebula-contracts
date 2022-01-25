use crate::contract::*;
use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::testing::{
    mock_env, mock_info, MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR,
};
use cosmwasm_std::*;
use cw20::BalanceResponse as Cw20BalanceResponse;
use cw20::{Cw20QueryMsg, TokenInfoResponse};
use nebula_protocol::{
    cluster::{InstantiateMsg, QueryMsg as ClusterQueryMsg},
    cluster_factory::ConfigResponse as FactoryConfigResponse,
    oracle::{PriceResponse, QueryMsg as OracleQueryMsg},
    penalty::{PenaltyCreateResponse, PenaltyRedeemResponse, QueryMsg as PenaltyQueryMsg},
};
use std::collections::HashMap;
use std::str::FromStr;
use terra_cosmwasm::*;

const DECIMAL_FRACTIONAL: Uint128 = Uint128::new(1_000_000_000u128);
pub fn decimal_division(a: Decimal, b: Decimal) -> Decimal {
    Decimal::from_ratio(DECIMAL_FRACTIONAL * a, b * DECIMAL_FRACTIONAL)
}

pub mod consts {
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
                    contract_addr: Addr::unchecked("mAAPL"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mGOOG"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mMSFT"),
                },
                amount: Uint128::new(20),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mNFLX"),
                },
                amount: Uint128::new(20),
            },
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
            astroport_factory: h("ts_factory"),
            token_code_id: 1,
            cluster_code_id: 1,
            base_denom: "uusd".to_string(),
            genesis_time: 1,
            distribution_schedule: vec![(1, 2, Uint128::from(123u128))],
        }
    }

    pub fn mint_response() -> PenaltyCreateResponse {
        PenaltyCreateResponse {
            create_tokens: Uint128::new(99),
            penalty: Uint128::new(1234),
            attributes: vec![attr("penalty", "1234")],
        }
    }

    pub fn asset_amounts() -> Vec<Asset> {
        vec![
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mAAPL"),
                },
                amount: Uint128::new(125_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mGOOG"),
                },
                amount: Uint128::zero(),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mMSFT"),
                },
                amount: Uint128::new(149_000_000),
            },
            Asset {
                info: AssetInfo::Token {
                    contract_addr: Addr::unchecked("mNFLX"),
                },
                amount: Uint128::new(50_090_272),
            },
        ]
    }
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
                    error: format!("Parsing query request: {:?}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.execute_query(&request)
    }
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
                    }) => match self.oracle_querier.assets.get(&base_asset.to_string()) {
                        Some(base_price) => {
                            match self.oracle_querier.assets.get(&quote_asset.to_string()) {
                                Some(quote_price) => SystemResult::Ok(ContractResult::from(
                                    to_binary(&PriceResponse {
                                        rate: decimal_division(*base_price, *quote_price),
                                        last_updated_base: u64::MAX,
                                        last_updated_quote: u64::MAX,
                                    }),
                                )),
                                None => SystemResult::Err(SystemError::InvalidRequest {
                                    error: "No oracle price exists".to_string(),
                                    request: msg.as_slice().into(),
                                }),
                            }
                        }
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
                                    create_asset_amounts: _,
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
                                    let response = PenaltyRedeemResponse {
                                        redeem_assets: vec![
                                            Uint128::new(99),
                                            Uint128::new(0),
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

#[derive(Default)]
pub struct PenaltyQuerier {
    pub create_tokens: Uint128,
    pub token_cost: Uint128,
    pub redeem_assets: Vec<Uint128>,
}

#[derive(Default)]
pub struct BalanceQuerier {
    // this lets us iterate over all pairs that match the first string

    // balances: denom -> account address -> amount
    pub balances: HashMap<String, HashMap<String, Uint128>>,
}

#[derive(Clone, Default)]
pub struct OracleQuerier {
    // this lets us iterate over all pairs that match the first string
    pub assets: HashMap<String, Decimal>,
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

    pub fn set_token<T>(&mut self, token_address: T, data: TokenData) -> &mut Self
    where
        T: Into<String>,
    {
        self.token_querier.tokens.insert(token_address.into(), data);
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

    pub fn set_mint_amount(&mut self, create_tokens: Uint128) -> &mut Self {
        self.penalty_querier.create_tokens = create_tokens;
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

pub fn mock_init() -> (OwnedDeps<MockStorage, MockApi, WasmMockQuerier>, Response) {
    let mut deps = mock_dependencies(&[]);
    deps = mock_querier_setup(deps);
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

/// sets up mock querier with basic setup
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
        )
        .set_token(
            "mGME",
            token_data(
                "Mirrored GME",
                "mGME",
                6,
                1_000_000_000_000,
                vec![(MOCK_CONTRACT_ADDR, 1_000_000)],
            ),
        )
        .set_token(
            "mGE",
            token_data(
                "Mirrored GE",
                "mGE",
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
        ("mGME", Decimal::from_str("1.0").unwrap()),
        ("mGE", Decimal::from_str("1.0").unwrap()),
    ]);

    deps
}
