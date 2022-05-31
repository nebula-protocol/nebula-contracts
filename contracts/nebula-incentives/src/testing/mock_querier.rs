use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::factory::PairType;
use astroport::pair::PoolResponse as AstroportPoolResponse;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, from_binary, from_slice, to_binary, Addr, Api, BalanceResponse, BankQuery, Binary,
    CanonicalAddr, Coin, ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use cw20::BalanceResponse as CW20BalanceResponse;
use nebula_protocol::cluster::ClusterStateResponse;
use nebula_protocol::cluster_factory::ClusterExistsResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

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
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    token_querier: TokenQuerier,
    balance_querier: BalanceQuerier,
    astroport_factory_querier: AstroportFactoryQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub(crate) fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct AstroportFactoryQuerier {
    pairs: HashMap<String, String>,
}

impl AstroportFactoryQuerier {
    pub fn new(pairs: &[(&String, &String)]) -> Self {
        AstroportFactoryQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(pairs: &[(&String, &String)]) -> HashMap<String, String> {
    let mut pairs_map: HashMap<String, String> = HashMap::new();
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), pair.to_string());
    }
    pairs_map
}

#[derive(Clone, Default)]
pub struct BalanceQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl BalanceQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        BalanceQuerier {
            balances: native_balances_to_map(balances),
        }
    }
}

pub(crate) fn native_balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (denom, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }
        balances_map.insert((**denom).to_string(), contract_balances_map);
    }
    balances_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair { asset_infos: [AssetInfo; 2] },
    ClusterState {},
    ClusterExists {},
    Pool {},
    Balance { address: String },
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
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
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => match from_binary(&msg)
                .unwrap()
            {
                QueryMsg::Pair { asset_infos } => {
                    let key = asset_infos[0].to_string() + asset_infos[1].to_string().as_str();
                    match self.astroport_factory_querier.pairs.get(&key) {
                        Some(v) => SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                            contract_addr: Addr::unchecked(v),
                            liquidity_token: Addr::unchecked("liquidity"),
                            asset_infos: [
                                AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                            ],
                            pair_type: PairType::Xyk {},
                        }))),
                        None => SystemResult::Err(SystemError::InvalidRequest {
                            error: "No pair info exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    }
                }
                QueryMsg::ClusterState {} => {
                    let response = ClusterStateResponse {
                        outstanding_balance_tokens: Uint128::new(1000),
                        prices: vec!["11.85".to_string(), "3.31".to_string()],
                        inv: vec![Uint128::new(110), Uint128::new(100), Uint128::new(95)],
                        penalty: "penalty".to_string(),
                        cluster_token: "cluster_token".to_string(),
                        target: vec![
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: Addr::unchecked("asset0000"),
                                },
                                amount: Uint128::new(100),
                            },
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: Addr::unchecked("asset0001"),
                                },
                                amount: Uint128::new(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "native_asset0000".to_string(),
                                },
                                amount: Uint128::new(100),
                            },
                        ],
                        cluster_contract_address: "cluster".to_string(),
                        active: true,
                    };
                    SystemResult::Ok(ContractResult::from(to_binary(&response)))
                }
                QueryMsg::ClusterExists {} => {
                    SystemResult::Ok(ContractResult::from(to_binary(&ClusterExistsResponse {
                        exists: true,
                    })))
                }
                QueryMsg::Pool {} => {
                    SystemResult::Ok(ContractResult::from(to_binary(&AstroportPoolResponse {
                        assets: [
                            Asset {
                                info: AssetInfo::Token {
                                    contract_addr: Addr::unchecked("cluster_token"),
                                },
                                amount: Uint128::new(100),
                            },
                            Asset {
                                info: AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                amount: Uint128::new(100),
                            },
                        ],
                        total_share: Uint128::new(10000),
                    })))
                }
                QueryMsg::Balance { address } => {
                    let balances: &HashMap<String, Uint128> =
                        match self.token_querier.balances.get(contract_addr) {
                            Some(balances) => balances,
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

                    let balance = match balances.get(&address) {
                        Some(v) => *v,
                        None => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&CW20BalanceResponse {
                                    balance: Uint128::zero(),
                                })
                                .unwrap(),
                            ));
                        }
                    };

                    SystemResult::Ok(ContractResult::Ok(
                        to_binary(&CW20BalanceResponse { balance }).unwrap(),
                    ))
                }
            },
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();
                let prefix_balance = to_length_prefixed(b"balance").to_vec();

                let balances: &HashMap<String, Uint128> =
                    match self.token_querier.balances.get(contract_addr) {
                        Some(balances) => balances,
                        None => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: format!(
                                    "No balance info exists for the contract {}",
                                    contract_addr
                                ),
                                request: key.into(),
                            })
                        }
                    };

                if key[..prefix_balance.len()].to_vec() == prefix_balance {
                    let key_address: &[u8] = &key[prefix_balance.len()..];
                    let address_raw: CanonicalAddr = CanonicalAddr::from(key_address);

                    let api: MockApi = MockApi::default();
                    let address: String = match api.addr_humanize(&address_raw) {
                        Ok(v) => v.to_string(),
                        Err(e) => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: format!("Parsing query request: {:?}", e),
                                request: key.into(),
                            })
                        }
                    };

                    let balance = match balances.get(&address) {
                        Some(v) => v,
                        None => {
                            return SystemResult::Err(SystemError::InvalidRequest {
                                error: "Balance not found".to_string(),
                                request: key.into(),
                            })
                        }
                    };

                    SystemResult::Ok(ContractResult::from(to_binary(&balance)))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            balance_querier: BalanceQuerier::default(),
            astroport_factory_querier: AstroportFactoryQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the astroport pair
    pub fn with_astroport_pairs(&mut self, pairs: &[(&String, &String)]) {
        self.astroport_factory_querier = AstroportFactoryQuerier::new(pairs);
    }

    // configure the bank
    pub fn with_native_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.balance_querier = BalanceQuerier::new(balances);
    }
}
