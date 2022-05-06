use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::factory::PairType;
use astroport::pair::PoolResponse as AstroportPoolResponse;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, from_binary, from_slice, to_binary, Addr, Coin, ContractResult, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use nebula_protocol::cluster::ClusterStateResponse;
use nebula_protocol::penalty::PenaltyNotionalResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use terra_cosmwasm::TerraQueryWrapper;

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

pub struct WasmMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    astroport_factory_querier: AstroportFactoryQuerier,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Pair {
        asset_infos: [AssetInfo; 2],
    },
    ClusterState {},
    Pool {},
    PenaltyQueryNotional {
        block_height: u64,
        inventory0: Vec<Uint128>,
        inventory1: Vec<Uint128>,
        asset_prices: Vec<String>,
        target_weights: Vec<Uint128>,
    },
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(&msg).unwrap() {
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
                QueryMsg::PenaltyQueryNotional {
                    block_height: _,
                    inventory0: _,
                    inventory1: _,
                    asset_prices: _,
                    target_weights: _,
                } => SystemResult::Ok(ContractResult::from(to_binary(&PenaltyNotionalResponse {
                    penalty: Uint128::new(4),
                    imbalance0: Uint128::new(100),
                    imbalance1: Uint128::new(51),
                    attributes: vec![attr("penalty", &format!("{}", 4))],
                }))),
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        WasmMockQuerier {
            base,
            astroport_factory_querier: AstroportFactoryQuerier::default(),
        }
    }

    // configure the astroport pair
    pub fn with_astroport_pairs(&mut self, pairs: &[(&String, &String)]) {
        self.astroport_factory_querier = AstroportFactoryQuerier::new(pairs);
    }
}
