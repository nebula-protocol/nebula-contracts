use astroport::asset::Asset;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read, Singleton};
use nebula_protocol::cluster::ClusterConfig;

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";
/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";
/// asset data: Vec<AssetData>
pub static ASSET_DATA_KEY: &[u8] = b"asset_data";

/// balance: Uint128
pub static PREFIX_BALANCE: &[u8] = b"balance";

//////////////////////////////////////////////////////////////////////
/// CONFIG
//////////////////////////////////////////////////////////////////////

pub fn config_store(storage: &mut dyn Storage) -> Singleton<ClusterConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<ClusterConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn store_config(storage: &mut dyn Storage, config: &ClusterConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

//////////////////////////////////////////////////////////////////////
/// ASSET DATA
//////////////////////////////////////////////////////////////////////

pub fn read_target_asset_data(storage: &dyn Storage) -> StdResult<Vec<Asset>> {
    singleton_read(storage, ASSET_DATA_KEY).load()
}

pub fn store_target_asset_data(storage: &mut dyn Storage, asset_data: &[Asset]) -> StdResult<()> {
    singleton(storage, ASSET_DATA_KEY).save(&asset_data.to_owned())
}

//////////////////////////////////////////////////////////////////////
/// ASSET BALANCE (INVENTORY)
//////////////////////////////////////////////////////////////////////

pub fn store_asset_balance(
    storage: &mut dyn Storage,
    asset: &str,
    inventory: &Uint128,
) -> StdResult<()> {
    bucket(storage, PREFIX_BALANCE).save(asset.as_bytes(), inventory)
}

pub fn read_asset_balance(storage: &dyn Storage, asset: &str) -> StdResult<Uint128> {
    Ok(bucket_read(storage, PREFIX_BALANCE)
        .load(asset.as_bytes())
        .unwrap_or_else(|_| Uint128::zero()))
}
