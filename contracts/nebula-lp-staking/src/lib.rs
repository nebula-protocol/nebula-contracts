pub mod contract;
mod math;
mod migration;
mod rewards;
mod staking;
mod state;

// Testing files
mod contract_test;
mod migration_test;
mod reward_test;
mod staking_test;
mod math_test;

#[cfg(target_arch = "wasm32")]
cosmwasm_std::create_entry_points_with_migration!(contract);