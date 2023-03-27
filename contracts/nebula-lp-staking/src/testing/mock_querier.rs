use std::marker::PhantomData;

use astroport::asset::{Asset, AssetInfo, PairInfo};
use astroport::factory::PairType;
use astroport::pair::PoolResponse;
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Addr, Coin, ContractResult, Decimal, Empty, OwnedDeps,
    Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery,
};
use cosmwasm_storage::to_length_prefixed;
use nebula_protocol::oracle::PriceResponse;
use serde::Deserialize;

pub struct WasmMockQuerier {
    base: MockQuerier,
    pair_addr: String,
    pool_assets: [Asset; 2],
    oracle_price: Decimal,
    token_balance: Uint128,
}

pub fn mock_dependencies_with_querier(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
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

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MockQueryMsg {
    Pair {
        asset_infos: [AssetInfo; 2],
    },
    Price {
        base_asset: String,
        quote_asset: String,
    },
    Pool {},
    Balance {
        address: String,
    },
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(msg).unwrap() {
                MockQueryMsg::Pair { asset_infos } => {
                    SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                        asset_infos,
                        contract_addr: Addr::unchecked(self.pair_addr.clone()),
                        liquidity_token: Addr::unchecked("lptoken"),
                        pair_type: PairType::Xyk {},
                    })))
                }
                MockQueryMsg::Pool {} => {
                    SystemResult::Ok(ContractResult::from(to_binary(&PoolResponse {
                        assets: self.pool_assets.clone(),
                        total_share: Uint128::zero(),
                    })))
                }
                MockQueryMsg::Price {
                    base_asset: _,
                    quote_asset: _,
                } => SystemResult::Ok(ContractResult::from(to_binary(&PriceResponse {
                    rate: self.oracle_price,
                    last_updated_base: 100,
                    last_updated_quote: 100,
                }))),
                MockQueryMsg::Balance { address: _ } => {
                    SystemResult::Ok(ContractResult::from(to_binary(&cw20::BalanceResponse {
                        balance: self.token_balance,
                    })))
                }
            },
            QueryRequest::Wasm(WasmQuery::Raw {
                contract_addr: _,
                key,
            }) => {
                let key: &[u8] = key.as_slice();
                let prefix_balance = to_length_prefixed(b"balance").to_vec();
                if key[..prefix_balance.len()].to_vec() == prefix_balance {
                    SystemResult::Ok(ContractResult::from(to_binary(&self.token_balance)))
                } else {
                    panic!("DO NOT ENTER HERE")
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            pair_addr: String::default(),
            pool_assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uasset".to_string(),
                    },
                    amount: Uint128::zero(),
                },
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: Addr::unchecked("asset"),
                    },
                    amount: Uint128::zero(),
                },
            ],
            oracle_price: Decimal::zero(),
            token_balance: Uint128::zero(),
        }
    }

    pub fn with_pair_info(&mut self, pair_addr: String) {
        self.pair_addr = pair_addr;
    }

    pub fn with_pool_assets(&mut self, pool_assets: [Asset; 2]) {
        self.pool_assets = pool_assets;
    }

    pub fn with_token_balance(&mut self, token_balance: Uint128) {
        self.token_balance = token_balance;
    }
}
