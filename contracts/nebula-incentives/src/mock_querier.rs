// use nebula_protocol::cluster::ClusterStateResponse;
// use nebula_protocol::cluster_factory::ClusterExistsResponse;
// use schemars::JsonSchema;
// use serde::{Deserialize, Serialize};

// use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
// use cosmwasm_std::{
//     coin, from_binary, from_slice, to_binary, Api, BalanceResponse, BankQuery, Binary,
//     CanonicalAddr, Coin, Decimal, Deps, QuerierResult, QueryRequest, SystemError, Uint128,
//     WasmQuery, Querier
// };
// use cosmwasm_storage::to_length_prefixed;

// use std::collections::HashMap;

// use terra_cosmwasm::{TaxCapResponse, TaxRateResponse, TerraQuery, TerraQueryWrapper, TerraRoute};
// use terraswap::asset::{Asset, AssetInfo, PairInfo};
// use terraswap::pair::PoolResponse as TerraswapPoolResponse;

// /// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
// /// this uses our CustomQuerier.
// pub fn mock_dependencies(canonical_length: usize, contract_balance: &[Coin]) -> Deps {
//     let contract_addr = MOCK_CONTRACT_ADDR.to_string();
//     let custom_querier: WasmMockQuerier = WasmMockQuerier::new(
//         MockQuerier::new(&[(&contract_addr, contract_balance)]),
//         MockApi::new(canonical_length),
//         canonical_length,
//     );

//     Deps {
//         storage: MockStorage::default(),
//         api: MockApi::new(canonical_length),
//         querier: custom_querier,
//     }
// }

// pub struct WasmMockQuerier {
//     base: MockQuerier<TerraQueryWrapper>,
//     token_querier: TokenQuerier,
//     balance_querier: BalanceQuerier,
//     tax_querier: TaxQuerier,
//     terraswap_factory_querier: TerraswapFactoryQuerier,
//     canonical_length: usize,
// }

// #[derive(Clone, Default)]
// pub struct TokenQuerier {
//     // this lets us iterate over all pairs that match the first string
//     balances: HashMap<String, HashMap<String, Uint128>>,
// }

// impl TokenQuerier {
//     pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
//         TokenQuerier {
//             balances: balances_to_map(balances),
//         }
//     }
// }

// pub(crate) fn balances_to_map(
//     balances: &[(&String, &[(&String, &Uint128)])],
// ) -> HashMap<String, HashMap<String, Uint128>> {
//     let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
//     for (contract_addr, balances) in balances.iter() {
//         let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
//         for (addr, balance) in balances.iter() {
//             contract_balances_map.insert((addr), **balance);
//         }

//         balances_map.insert((contract_addr), contract_balances_map);
//     }
//     balances_map
// }

// #[derive(Clone, Default)]
// pub struct TaxQuerier {
//     rate: Decimal,
//     // this lets us iterate over all pairs that match the first string
//     caps: HashMap<String, Uint128>,
// }

// impl TaxQuerier {
//     pub fn new(rate: Decimal, caps: &[(&String, &Uint128)]) -> Self {
//         TaxQuerier {
//             rate,
//             caps: caps_to_map(caps),
//         }
//     }
// }

// pub(crate) fn caps_to_map(caps: &[(&String, &Uint128)]) -> HashMap<String, Uint128> {
//     let mut owner_map: HashMap<String, Uint128> = HashMap::new();
//     for (denom, cap) in caps.iter() {
//         owner_map.insert(denom.to_string(), **cap);
//     }
//     owner_map
// }

// #[derive(Clone, Default)]
// pub struct TerraswapFactoryQuerier {
//     pairs: HashMap<String, String>,
// }

// impl TerraswapFactoryQuerier {
//     pub fn new(pairs: &[(&String, &String)]) -> Self {
//         TerraswapFactoryQuerier {
//             pairs: pairs_to_map(pairs),
//         }
//     }
// }

// pub(crate) fn pairs_to_map(pairs: &[(&String, &String)]) -> HashMap<String, String> {
//     let mut pairs_map: HashMap<String, String> = HashMap::new();
//     for (key, pair) in pairs.iter() {
//         pairs_map.insert(key.to_string(), (pair));
//     }
//     pairs_map
// }

// #[derive(Clone, Default)]
// pub struct BalanceQuerier {
//     // this lets us iterate over all pairs that match the first string
//     balances: HashMap<String, HashMap<String, Uint128>>,
// }

// impl BalanceQuerier {
//     pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
//         BalanceQuerier {
//             balances: native_balances_to_map(balances),
//         }
//     }
// }

// pub(crate) fn native_balances_to_map(
//     balances: &[(&String, &[(&String, &Uint128)])],
// ) -> HashMap<String, HashMap<String, Uint128>> {
//     let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
//     for (denom, balances) in balances.iter() {
//         let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
//         for (addr, balance) in balances.iter() {
//             contract_balances_map.insert((addr), **balance);
//         }
//         balances_map.insert((**denom).to_string(), contract_balances_map);
//     }
//     balances_map
// }

// impl Querier for WasmMockQuerier {
//     fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
//         // MockQuerier doesn't support Custom, so we ignore it completely here
//         let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
//             Ok(v) => v,
//             Err(e) => {
//                 return Err(SystemError::InvalidRequest {
//                     error: format!("Parsing query request: {}", e),
//                     request: bin_request.into(),
//                 })
//             }
//         };
//         self.execute_query(&request)
//     }
// }

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub enum QueryMsg {
//     Pair { asset_infos: [AssetInfo; 2] },
//     ClusterState { cluster_contract_address: String },
//     ClusterExists {},
//     Pool {},
// }

// impl WasmMockQuerier {
//     pub fn execute_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
//         match &request {
//             QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
//                 if route == &TerraRoute::Treasury {
//                     match query_data {
//                         TerraQuery::TaxRate {} => {
//                             let res = TaxRateResponse {
//                                 rate: self.tax_querier.rate,
//                             };
//                             Ok(to_binary(&res))
//                         }
//                         TerraQuery::TaxCap { denom } => {
//                             let cap = self
//                                 .tax_querier
//                                 .caps
//                                 .get(denom)
//                                 .copied()
//                                 .unwrap_or_default();
//                             let res = TaxCapResponse { cap };
//                             Ok(to_binary(&res))
//                         }
//                         _ => panic!("DO NOT ENTER HERE"),
//                     }
//                 } else {
//                     panic!("DO NOT ENTER HERE")
//                 }
//             }
//             QueryRequest::Bank(BankQuery::Balance { address, denom }) => {
//                 // Do for native
//                 let denom_data = match self.balance_querier.balances.get(denom) {
//                     Some(v) => v,
//                     None => {
//                         return Err(SystemError::InvalidRequest {
//                             error: format!("Denom not found in balances"),
//                             request: Binary(vec![]),
//                         })
//                     }
//                 };
//                 let balance = match denom_data.get(&address) {
//                     Some(v) => v,
//                     None => &Uint128::zero(),
//                 };
//                 Ok(to_binary(&BalanceResponse {
//                     amount: coin(balance.u128(), denom),
//                 }))
//             }
//             QueryRequest::Wasm(WasmQuery::Smart {
//                 contract_addr: _,
//                 msg,
//             }) => match from_binary(&msg).unwrap() {
//                 QueryMsg::Pair { asset_infos } => {
//                     let key = asset_infos[0].to_string() + asset_infos[1].to_string().as_str();
//                     match self.terraswap_factory_querier.pairs.get(&key) {
//                         Some(v) => Ok(to_binary(&PairInfo {
//                             contract_addr: v.clone(),
//                             liquidity_token: ("liquidity"),
//                             asset_infos: [
//                                 AssetInfo::NativeToken {
//                                     denom: "uusd".to_string(),
//                                 },
//                                 AssetInfo::NativeToken {
//                                     denom: "uusd".to_string(),
//                                 },
//                             ],
//                         })),
//                         None => Err(SystemError::InvalidRequest {
//                             error: "No pair info exists".to_string(),
//                             request: msg.as_slice().into(),
//                         }),
//                     }
//                 }
//                 QueryMsg::ClusterState {
//                     cluster_contract_address,
//                 } => {
//                     let response = ClusterStateResponse {
//                         outstanding_balance_tokens: Uint128::new(1000),
//                         prices: vec!["11.85".to_string(), "3.31".to_string()],
//                         inv: vec![Uint128::new(110), Uint128::new(100), Uint128::new(95)],
//                         penalty: ("penalty"),
//                         cluster_token: ("cluster_token"),
//                         target: vec![
//                             Asset {
//                                 info: AssetInfo::Token {
//                                     contract_addr: ("asset0000"),
//                                 },
//                                 amount: Uint128::new(100),
//                             },
//                             Asset {
//                                 info: AssetInfo::Token {
//                                     contract_addr: ("asset0001"),
//                                 },
//                                 amount: Uint128::new(100),
//                             },
//                             Asset {
//                                 info: AssetInfo::NativeToken {
//                                     denom: "native_asset0000".to_string(),
//                                 },
//                                 amount: Uint128::new(100),
//                             },
//                         ],
//                         cluster_contract_address: cluster_contract_address,
//                         active: true,
//                     };
//                     Ok(to_binary(&response))
//                 }
//                 QueryMsg::ClusterExists {} => {
//                     Ok(to_binary(&ClusterExistsResponse { exists: true }))
//                 }
//                 QueryMsg::Pool {} => Ok(to_binary(&TerraswapPoolResponse {
//                     assets: [
//                         Asset {
//                             info: AssetInfo::Token {
//                                 contract_addr: ("cluster_token"),
//                             },
//                             amount: Uint128::new(100),
//                         },
//                         Asset {
//                             info: AssetInfo::NativeToken {
//                                 denom: "uusd".to_string(),
//                             },
//                             amount: Uint128::new(100),
//                         },
//                     ],
//                     total_share: Uint128::new(10000),
//                 })),
//             },
//             QueryRequest::Wasm(WasmQuery::Raw { contract_addr, key }) => {
//                 let key: &[u8] = key.as_slice();
//                 let prefix_balance = to_length_prefixed(b"balance").to_vec();

//                 let balances: &HashMap<String, Uint128> =
//                     match self.token_querier.balances.get(contract_addr) {
//                         Some(balances) => balances,
//                         None => {
//                             return Err(SystemError::InvalidRequest {
//                                 error: format!(
//                                     "No balance info exists for the contract {}",
//                                     contract_addr
//                                 ),
//                                 request: key.into(),
//                             })
//                         }
//                     };

//                 if key[..prefix_balance.len()].to_vec() == prefix_balance {
//                     let key_address: &[u8] = &key[prefix_balance.len()..];
//                     let address_raw: CanonicalAddr = CanonicalAddr::from(key_address);

//                     let api: MockApi = MockApi::new(self.canonical_length);
//                     let address: String = match api.human_address(&address_raw) {
//                         Ok(v) => v,
//                         Err(e) => {
//                             return Err(SystemError::InvalidRequest {
//                                 error: format!("Parsing query request: {}", e),
//                                 request: key.into(),
//                             })
//                         }
//                     };

//                     let balance = match balances.get(&address) {
//                         Some(v) => v,
//                         None => {
//                             return Err(SystemError::InvalidRequest {
//                                 error: "Balance not found".to_string(),
//                                 request: key.into(),
//                             })
//                         }
//                     };

//                     Ok(to_binary(&to_binary(&balance).unwrap()))
//                 } else {
//                     panic!("DO NOT ENTER HERE")
//                 }
//             }
//             _ => self.base.execute_query(request),
//         }
//     }
// }

// impl WasmMockQuerier {
//     pub fn new(
//         base: MockQuerier<TerraQueryWrapper>,
//         _api: &dyn Api,
//         canonical_length: usize,
//     ) -> Self {
//         WasmMockQuerier {
//             base,
//             token_querier: TokenQuerier::default(),
//             balance_querier: BalanceQuerier::default(),
//             tax_querier: TaxQuerier::default(),
//             terraswap_factory_querier: TerraswapFactoryQuerier::default(),
//             canonical_length,
//         }
//     }

//     // configure the mint whitelist mock querier
//     pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
//         self.token_querier = TokenQuerier::new(balances);
//     }

//     // configure the token owner mock querier
//     pub fn with_tax(&mut self, rate: Decimal, caps: &[(&String, &Uint128)]) {
//         self.tax_querier = TaxQuerier::new(rate, caps);
//     }

//     // configure the terraswap pair
//     pub fn with_terraswap_pairs(&mut self, pairs: &[(&String, &String)]) {
//         self.terraswap_factory_querier = TerraswapFactoryQuerier::new(pairs);
//     }

//     // configure the bank
//     pub fn with_native_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
//         self.balance_querier = BalanceQuerier::new(balances);
//     }
// }
