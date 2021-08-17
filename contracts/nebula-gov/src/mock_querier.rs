// use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
// use cosmwasm_std::{
//     from_binary, from_slice, to_binary, Coin, Deps, Empty, Querier, QuerierResult, QueryRequest,
//     SystemError, Uint128, WasmQuery,
// };
// use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
// use std::collections::HashMap;

// /// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
// /// this uses our CustomQuerier.
// pub fn mock_dependencies(
//     canonical_length: usize,
//     contract_balance: &[Coin],
// ) -> Deps<MockStorage, MockApi, WasmMockQuerier> {
//     let contract_addr = MOCK_CONTRACT_ADDR;
//     let custom_querier: WasmMockQuerier =
//         WasmMockQuerier::new(MockQuerier::new(&[(&contract_addr, contract_balance)]));

//     Deps {
//         storage: MockStorage::default(),
//         api: MockApi::new(canonical_length),
//         querier: custom_querier,
//     }
// }

// pub struct WasmMockQuerier {
//     base: MockQuerier<Empty>,
//     token_querier: TokenQuerier,
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
//             contract_balances_map.insert(addr, **balance);
//         }

//         balances_map.insert(contract_addr, contract_balances_map);
//     }
//     balances_map
// }

// impl Querier for WasmMockQuerier {
//     fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
//         // MockQuerier doesn't support Custom, so we ignore it completely here
//         let request: QueryRequest<Empty> = match from_slice(bin_request) {
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

// impl WasmMockQuerier {
//     pub fn execute_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
//         match &request {
//             QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
//                 match from_binary(&msg).unwrap() {
//                     Cw20QueryMsg::TokenInfo {} => {
//                         let balances: &HashMap<String, Uint128> =
//                             match self.token_querier.balances.get(contract_addr) {
//                                 Some(balances) => balances,
//                                 None => {
//                                     return Err(SystemError::InvalidRequest {
//                                         error: format!(
//                                             "No balance info exists for the contract {}",
//                                             contract_addr
//                                         ),
//                                         request: msg.as_slice().into(),
//                                     })
//                                 }
//                             };

//                         let mut total_supply = Uint128::zero();

//                         for balance in balances {
//                             total_supply += *balance.1;
//                         }

//                         Ok(to_binary(&TokenInfoResponse {
//                             name: "mAAPL".to_string(),
//                             symbol: "mAAPL".to_string(),
//                             decimals: 6,
//                             total_supply: total_supply,
//                         }))
//                     }
//                     Cw20QueryMsg::Balance { address } => {
//                         let balances: &HashMap<String, Uint128> =
//                             match self.token_querier.balances.get(contract_addr) {
//                                 Some(balances) => balances,
//                                 None => {
//                                     return Err(SystemError::InvalidRequest {
//                                         error: format!(
//                                             "No balance info exists for the contract {}",
//                                             contract_addr
//                                         ),
//                                         request: msg.as_slice().into(),
//                                     })
//                                 }
//                             };

//                         let balance = match balances.get(&address) {
//                             Some(v) => *v,
//                             None => {
//                                 return Ok(to_binary(&Cw20BalanceResponse {
//                                     balance: Uint128::zero(),
//                                 }));
//                             }
//                         };

//                         Ok(to_binary(&Cw20BalanceResponse { balance }))
//                     }
//                     _ => panic!("DO NOT ENTER HERE"),
//                 }
//             }
//             _ => self.base.execute_query(request),
//         }
//     }
// }

// impl WasmMockQuerier {
//     pub fn new(base: MockQuerier<Empty>) -> Self {
//         WasmMockQuerier {
//             base,
//             token_querier: TokenQuerier::default(),
//         }
//     }

//     // configure the mint whitelist mock querier
//     pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
//         self.token_querier = TokenQuerier::new(balances);
//     }
// }
