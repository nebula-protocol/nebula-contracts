use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Singleton};
use nebula_protocol::cluster::ClusterConfig;
use terraswap::asset::Asset;

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";

/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";

/// asset data: Vec<AssetData>
pub static ASSET_DATA_KEY: &[u8] = b"asset_data";

pub fn config_store(storage: &mut dyn Storage) -> Singleton<Storage, ClusterConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<ClusterConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config(storage: &mut dyn Storage, config: &ClusterConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

pub fn read_target_asset_data(storage: &dyn Storage) -> StdResult<Vec<Asset>> {
    singleton_read(storage, ASSET_DATA_KEY).load()
}

pub fn save_target_asset_data(storage: &mut dyn Storage, asset_data: &Vec<Asset>) -> StdResult<()> {
    singleton(storage, ASSET_DATA_KEY).save(asset_data)
}

pub fn read_target(storage: &dyn Storage) -> StdResult<Vec<u32>> {
    singleton_read(storage, TARGET_KEY).load()
}

pub fn save_target(storage: &mut dyn Storage, target: &Vec<u32>) -> StdResult<()> {
    singleton(storage, TARGET_KEY).save(target)
}
