use cosmwasm_std::{Addr, Binary, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("config");
pub const MIGRATION_RECORDS_BY_TIME: Map<u64, MigrationRecord> = Map::new("migration_records");
pub const AUTH_RECORDS_BY_TIME: Map<u64, AuthRecord> = Map::new("auth_records");
pub const AUTH_LIST: Map<Addr, u64> = Map::new("auth_list");

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub admin_claim_period: u64,
}

//////////////////////////////////////////////////////////////////////
/// AUTH RECORD
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthRecord {
    pub address: Addr,
    pub start_time: u64,
    pub end_time: u64,
}

pub fn is_addr_authorized(storage: &dyn Storage, address: Addr, current_time: u64) -> bool {
    match AUTH_LIST.load(storage, address) {
        Ok(claim_end) => claim_end >= current_time,
        Err(_) => false,
    }
}

//////////////////////////////////////////////////////////////////////
/// MIGRATION RECORD
//////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationRecord {
    pub executor: Addr,
    pub time: u64,
    pub migrations: Vec<(Addr, u64, Binary)>,
}
