use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Api, CanonicalAddr, Coin, Decimal, Deps, Empty,
    QuerierResult, QueryRequest, SystemError, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;

use crate::querier::MintAssetConfig;
use std::collections::HashMap;
use terraswap::asset::{AssetInfo, PairInfo};

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    canonical_length: usize,
    contract_balance: &[Coin],
) -> Deps<MockStorage, MockApi, WasmMockQuerier> {
    let contract_addr = (MOCK_CONTRACT_ADDR);
    let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
        MockQuerier::new(&[(&contract_addr, contract_balance)]),
        MockApi::new(canonical_length),
        canonical_length,
    );

    Deps {
        storage: MockStorage::default(),
        api: MockApi::new(canonical_length),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    terraswap_factory_querier: TerraswapFactory,
    oracle_querier: Oracle,
    mint_querier: Mint,
    canonical_length: usize,
}

#[derive(Clone, Default)]
pub struct TerraswapFactoryQuerier {
    pairs: HashMap<String, String>,
}

impl TerraswapFactoryQuerier {
    pub fn new(pairs: &[(&String, &String)]) -> Self {
        TerraswapFactoryQuerier {
            pairs: pairs_to_map(pairs),
        }
    }
}

pub(crate) fn pairs_to_map(pairs: &[(&String, &String)]) -> HashMap<String, String> {
    let mut pairs_map: HashMap<String, String> = HashMap::new();
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), (pair));
    }
    pairs_map
}

#[derive(Clone, Default)]
pub struct OracleQuerier {
    feeders: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct MintQuerier {
    configs: HashMap<String, (Decimal, Decimal, Option<Decimal>)>,
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
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
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(&msg).unwrap() {
                QueryMsg::Pair { asset_infos } => {
                    let key = asset_infos[0].to_string() + asset_infos[1].to_string().as_str();
                    match self.terraswap_factory_querier.pairs.get(&key) {
                        Some(v) => Ok(to_binary(&PairInfo {
                            contract_addr: ("pair"),
                            liquidity_token: v.clone(),
                            asset_infos: [
                                AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                                AssetInfo::NativeToken {
                                    denom: "uusd".to_string(),
                                },
                            ],
                        })),
                        None => Err(SystemError::InvalidRequest {
                            error: "No pair info exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    }
                }
            },
            QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
                let key: &[u8] = key.as_slice();
                let prefix_asset_config = to_length_prefixed(b"asset_config").to_vec();
                let prefix_feeder = to_length_prefixed(b"feeder").to_vec();

                let api: MockApi = MockApi::new(self.canonical_length);
                if key.len() > prefix_feeder.len()
                    && key[..prefix_feeder.len()].to_vec() == prefix_feeder
                {
                    let api: MockApi = MockApi::new(self.canonical_length);
                    let rest_key: &[u8] = &key[prefix_feeder.len()..];

                    if contract_addr == &("oracle0000") {
                        let asset_token: String = api
                            .human_address(&(CanonicalAddr::from(rest_key.to_vec())))
                            .unwrap();

                        let feeder = match self.oracle_querier.feeders.get(&asset_token) {
                            Some(v) => v,
                            None => {
                                return Err(SystemError::InvalidRequest {
                                    error: format!(
                                        "Oracle Feeder is not found for {}",
                                        asset_token
                                    ),
                                    request: key.into(),
                                })
                            }
                        };

                        Ok(to_binary(
                            &to_binary(&api.canonical_address(&feeder).unwrap()).unwrap(),
                        ))
                    } else {
                        panic!("DO NOT ENTER HERE")
                    }
                } else if key.len() > prefix_asset_config.len()
                    && key[..prefix_asset_config.len()].to_vec() == prefix_asset_config
                {
                    let rest_key: &[u8] = &key[prefix_asset_config.len()..];
                    let asset_token: String = api
                        .human_address(&(CanonicalAddr::from(rest_key.to_vec())))
                        .unwrap();

                    let config = match self.mint_querier.configs.get(&asset_token) {
                        Some(v) => v,
                        None => {
                            return Err(SystemError::InvalidRequest {
                                error: format!("Mint Config is not found for {}", asset_token),
                                request: key.into(),
                            })
                        }
                    };

                    Ok(to_binary(
                        &to_binary(&MintAssetConfig {
                            token: api.canonical_address(&asset_token).unwrap(),
                            auction_discount: config.0,
                            min_collateral_ratio: config.1,
                            min_collateral_ratio_after_migration: config.2,
                        })
                        .unwrap(),
                    ))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.execute_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>, _api: &dyn Api, canonical_length: usize) -> Self {
        WasmMockQuerier {
            base,
            terraswap_factory_querier: TerraswapFactoryQuerier::default(),
            mint_querier: MintQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            canonical_length,
        }
    }

    // configure the terraswap pair
    pub fn with_terraswap_pairs(&mut self, pairs: &[(&String, &String)]) {
        self.terraswap_factory_querier = TerraswapFactoryQuerier::new(pairs);
    }
}
