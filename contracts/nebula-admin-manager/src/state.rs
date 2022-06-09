use cosmwasm_std::{Addr, Binary, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const MIGRATION_RECORDS_BY_HEIGHT: Map<u64, MigrationRecord> = Map::new("migration_records");
pub const AUTH_RECORDS_BY_HEIGHT: Map<u64, AuthRecord> = Map::new("auth_records");
pub const AUTH_LIST: Map<Addr, u64> = Map::new("auth_list");

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// This structure holds the admin manager contract parameters
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the admin manager contract
    pub owner: Addr,
    /// The duration of admin privilege delegation to a defined address when `AuthorizeClaim` occurs.
    pub admin_claim_period: u64,
}

//////////////////////////////////////////////////////////////////////
/// AUTH RECORD
//////////////////////////////////////////////////////////////////////

/// ## Description
/// This structure holds an address and its related authorization block height.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthRecord {
    /// Address of an authorized account
    pub address: Addr,
    /// Start block height of the authorized period
    pub start_height: u64,
    /// End block height of the authorized period
    pub end_height: u64,
}

pub fn is_addr_authorized(storage: &dyn Storage, address: Addr, current_height: u64) -> bool {
    match AUTH_LIST.load(storage, address) {
        Ok(claim_end) => claim_end >= current_height,
        Err(_) => false,
    }
}

//////////////////////////////////////////////////////////////////////
/// MIGRATION RECORD
//////////////////////////////////////////////////////////////////////

/// ## Description
/// This structure holds records for each `ExecuteMigrations` execution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationRecord {
    /// Address of the transaction executor
    pub executor: Addr,
    /// Block height of the execution
    pub height: u64,
    /// A list of migrations and their details in the execution.
    /// Each tuple contains (contract_address, code_id, MigrateMsg)
    pub migrations: Vec<(Addr, u64, Binary)>,
}
