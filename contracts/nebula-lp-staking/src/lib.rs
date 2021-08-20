pub mod contract;
mod rewards;
mod staking;
mod state;

// Testing files
mod contract_test;
#[cfg(test)]
mod mock_querier;
mod reward_test;
mod staking_test;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points_with_migration!(contract);
