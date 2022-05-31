use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Decimal, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use tefi_oracle::hub::PriceResponse;

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
    pub base: MockQuerier,
    pub terra_oracle_querier: TerraOracleQuerier,
    pub tefi_oracle_querier: TefiOracleQuerier,
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

#[derive(Default)]
pub struct TerraOracleQuerier {
    pub denoms: HashMap<String, Decimal>,
}

#[derive(Clone, Default)]
pub struct TefiOracleQuerier {
    pub assets: HashMap<String, Decimal>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Price {
        asset_token: String,
        timeframe: Option<u64>,
    },
}

impl WasmMockQuerier {
    pub fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(&msg).unwrap() {
                QueryMsg::Price { asset_token, .. } => match self
                    .tefi_oracle_querier
                    .assets
                    .get(&asset_token.to_string())
                {
                    Some(price) => {
                        SystemResult::Ok(ContractResult::from(to_binary(&PriceResponse {
                            rate: price.clone(),
                            last_updated: u64::MAX,
                        })))
                    }
                    None => SystemResult::Err(SystemError::InvalidRequest {
                        error: "No oracle price exists".to_string(),
                        request: msg.as_slice().into(),
                    }),
                },
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            tefi_oracle_querier: TefiOracleQuerier::default(),
            terra_oracle_querier: TerraOracleQuerier::default(),
        }
    }

    pub fn set_terra_oracle_price(&mut self, native_denom: String, price: Decimal) -> &mut Self {
        self.terra_oracle_querier.denoms.insert(native_denom, price);
        self
    }

    pub fn set_terra_oracle_prices<T, U>(&mut self, price_data: T) -> &mut Self
    where
        T: IntoIterator<Item = (U, Decimal)>,
        U: ToString,
    {
        for (denom, price) in price_data.into_iter() {
            self.set_terra_oracle_price(denom.to_string(), price);
        }
        self
    }

    pub fn set_tefi_oracle_price(&mut self, asset_address: String, price: Decimal) -> &mut Self {
        self.tefi_oracle_querier.assets.insert(asset_address, price);
        self
    }

    pub fn set_tefi_oracle_prices<T, U>(&mut self, price_data: T) -> &mut Self
    where
        T: IntoIterator<Item = (U, Decimal)>,
        U: ToString,
    {
        for (asset, price) in price_data.into_iter() {
            self.set_tefi_oracle_price(asset.to_string(), price);
        }
        self
    }
}
