use cosmwasm_std::{Addr, StdError, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// config: Config
pub static KEY_CONFIG: &[u8] = b"config";
/// native_map: Map<Denom, Symbol>
pub static KEY_NATIVE_MAP: &[u8] = b"native_map";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

/// ## Description
/// A custom struct for storing oracle contract setting.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner of the contract
    pub owner: Addr,
    /// TeFi oracle hub contract
    pub oracle_addr: Addr,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

//////////////////////////////////////////////////////////////////////
/// NATIVE MAP
/// ## Description
/// A map storing denom and symbol for supported native token.
//////////////////////////////////////////////////////////////////////

pub fn store_native_map(
    storage: &mut dyn Storage,
    denom: &str,
    symbol: String,
) -> StdResult<()> {
    let mut native_map_bucket: Bucket<String> = Bucket::new(storage, KEY_NATIVE_MAP);
    native_map_bucket.save(denom.as_bytes(), &symbol)
}

pub fn read_native_map(storage: &dyn Storage, denom: &str) -> StdResult<String> {
    let native_map_bucket: ReadonlyBucket<String> = ReadonlyBucket::new(storage, KEY_NATIVE_MAP);
    match native_map_bucket.load(denom.as_bytes()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("No native map stored")),
    }
}
