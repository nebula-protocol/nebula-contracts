use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read, Singleton};
use nebula_protocol::cluster::ClusterConfig;
use terraswap::asset::{Asset, AssetInfo};

/// config: ClusterConfig
pub static CONFIG_KEY: &[u8] = b"config";

/// target: Vec<u32>
pub static TARGET_KEY: &[u8] = b"target";

/// asset data: Vec<AssetData>
pub static ASSET_DATA_KEY: &[u8] = b"asset_data";

pub fn config_store<S: Storage>(storage: &mut S) -> Singleton<S, ClusterConfig> {
    singleton(storage, CONFIG_KEY)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<ClusterConfig> {
    singleton_read(storage, CONFIG_KEY).load()
}

pub fn save_config<S: Storage>(storage: &mut S, config: &ClusterConfig) -> StdResult<()> {
    singleton(storage, CONFIG_KEY).save(config)
}

pub fn read_target_asset_data<S: Storage>(storage: &S) -> StdResult<Vec<Asset>> {
    singleton_read(storage, ASSET_DATA_KEY).load()
}

pub fn save_target_asset_data<S: Storage>(
    storage: &mut S,
    asset_data: &Vec<Asset>,
) -> StdResult<()> {
    singleton(storage, ASSET_DATA_KEY).save(asset_data)
}

pub fn read_target<S: Storage>(storage: &S) -> StdResult<Vec<u32>> {
    singleton_read(storage, TARGET_KEY).load()
}

pub fn save_target<S: Storage>(storage: &mut S, target: &Vec<u32>) -> StdResult<()> {
    singleton(storage, TARGET_KEY).save(target)
}
