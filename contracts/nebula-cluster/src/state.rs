use cosmwasm_std::{StdResult, Storage, Uint128};
use cosmwasm_storage::{bucket, bucket_read, singleton, singleton_read, Singleton};
use nebula_protocol::cluster::ClusterConfig;
use astroport::asset::Asset;

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";

/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";

/// asset data: Vec<AssetData>
pub static ASSET_DATA_KEY: &[u8] = b"asset_data";

pub static PREFIX_BALANCE: &[u8] = b"balance";

pub fn config_store(storage: &mut dyn Storage) -> Singleton<ClusterConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<ClusterConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn store_config(storage: &mut dyn Storage, config: &ClusterConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

pub fn read_target_asset_data(storage: &dyn Storage) -> StdResult<Vec<Asset>> {
    singleton_read(storage, ASSET_DATA_KEY).load()
}

pub fn store_target_asset_data(storage: &mut dyn Storage, asset_data: &Vec<Asset>) -> StdResult<()> {
    singleton(storage, ASSET_DATA_KEY).save(asset_data)
}

pub fn read_target(storage: &dyn Storage) -> StdResult<Vec<u32>> {
    singleton_read(storage, TARGET_KEY).load()
}

pub fn store_target(storage: &mut dyn Storage, target: &Vec<u32>) -> StdResult<()> {
    singleton(storage, TARGET_KEY).save(target)
}

pub fn store_asset_balance(
    storage: &mut dyn Storage,
    asset: &String,
    inventory: &Uint128,
) -> StdResult<()> {
    bucket(storage, PREFIX_BALANCE).save(asset.as_bytes(), inventory)
}

pub fn read_asset_balance(storage: &dyn Storage, asset: &String) -> StdResult<Uint128> {
    Ok(bucket_read(storage, PREFIX_BALANCE)
        .load(asset.as_bytes())
        .unwrap_or(Uint128::zero()))
}
