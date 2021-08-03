pub mod contract;
mod mock_querier;
mod querier;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points!(contract);
