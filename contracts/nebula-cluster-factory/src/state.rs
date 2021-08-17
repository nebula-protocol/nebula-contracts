use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Order, StdError, StdResult, Storage, Uint128};
use cosmwasm_storage::{singleton, singleton_read, Bucket, ReadonlyBucket, Singleton};

use nebula_protocol::cluster_factory::Params;

static KEY_CONFIG: &[u8] = b"config";
static KEY_PARAMS: &[u8] = b"params";
static KEY_TOTAL_WEIGHT: &[u8] = b"total_weight";
static KEY_LAST_DISTRIBUTED: &[u8] = b"last_distributed";

static PREFIX_WEIGHT: &[u8] = b"weight";
static PREFIX_CLUSTERS: &[u8] = b"clusters";
static PREFIX_TMP_CLUSTER: &[u8] = b"tmp_clusters";
static PREFIX_TMP_ASSET: &[u8] = b"tmp_asset";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub nebula_token: String,
    pub terraswap_factory: String,
    pub staking_contract: String,
    pub commission_collector: String,
    pub protocol_fee_rate: String,
    pub token_code_id: u64, // used to create asset token
    pub cluster_code_id: u64,
    pub base_denom: String,
    pub genesis_time: u64,
    pub distribution_schedule: Vec<(u64, u64, Uint128)>, // [[start_time, end_time, distribution_amount], [], ...]
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn cluster_exists(storage: &dyn Storage, contract_addr: &String) -> StdResult<bool> {
    match ReadonlyBucket::new(storage, PREFIX_CLUSTERS).load(&contract_addr.as_str().as_bytes()) {
        Ok(res) => Ok(res),
        Err(_) => Ok(false),
    }
}

pub fn get_cluster_data(storage: &dyn Storage) -> StdResult<Vec<(String, bool)>> {
    let cluster_bucket: ReadonlyBucket<bool> = ReadonlyBucket::new(storage, PREFIX_CLUSTERS);

    cluster_bucket
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (k, b) = item?;
            Ok((
                (unsafe { std::str::from_utf8_unchecked(&k) }).to_string(),
                b,
            ))
        })
        .collect::<StdResult<Vec<(String, bool)>>>()
}

pub fn store_tmp_cluster(storage: &mut dyn Storage, contract_addr: &String) -> StdResult<()> {
    singleton(storage, PREFIX_TMP_ASSET).save(contract_addr)
}

pub fn read_tmp_asset(storage: &dyn Storage) -> StdResult<String> {
    singleton_read(storage, PREFIX_TMP_ASSET).load()
}

pub fn store_tmp_asset(storage: &mut dyn Storage, contract_addr: &String) -> StdResult<()> {
    singleton(storage, PREFIX_TMP_CLUSTER).save(contract_addr)
}

pub fn read_tmp_cluster(storage: &dyn Storage) -> StdResult<String> {
    singleton_read(storage, PREFIX_TMP_CLUSTER).load()
}

pub fn record_cluster(storage: &mut dyn Storage, contract_addr: &String) -> StdResult<()> {
    Bucket::new(storage, PREFIX_CLUSTERS).save(&contract_addr.as_str().as_bytes(), &true)
}

pub fn deactivate_cluster(storage: &mut dyn Storage, contract_addr: &String) -> StdResult<()> {
    Bucket::new(storage, PREFIX_CLUSTERS).save(&contract_addr.as_str().as_bytes(), &false)
}

pub fn store_params(storage: &mut dyn Storage, init_data: &Params) -> StdResult<()> {
    singleton(storage, KEY_PARAMS).save(init_data)
}

pub fn remove_params(storage: &mut dyn Storage) {
    let mut store: Singleton<Params> = singleton(storage, KEY_PARAMS);
    store.remove()
}

pub fn read_params(storage: &dyn Storage) -> StdResult<Params> {
    singleton_read(storage, KEY_PARAMS).load()
}

pub fn store_total_weight(storage: &mut dyn Storage, total_weight: u32) -> StdResult<()> {
    singleton(storage, KEY_TOTAL_WEIGHT).save(&total_weight)
}

pub fn increase_total_weight(storage: &mut dyn Storage, weight_increase: u32) -> StdResult<u32> {
    let mut store: Singleton<u32> = singleton(storage, KEY_TOTAL_WEIGHT);
    store.update(|total_weight| Ok(total_weight + weight_increase))
}

pub fn decrease_total_weight(storage: &mut dyn Storage, weight_decrease: u32) -> StdResult<u32> {
    let mut store: Singleton<u32> = singleton(storage, KEY_TOTAL_WEIGHT);
    store.update(|total_weight| Ok(total_weight - weight_decrease))
}

pub fn read_total_weight(storage: &dyn Storage) -> StdResult<u32> {
    singleton_read(storage, KEY_TOTAL_WEIGHT).load()
}

pub fn store_last_distributed(storage: &mut dyn Storage, last_distributed: u64) -> StdResult<()> {
    let mut store: Singleton<u64> = singleton(storage, KEY_LAST_DISTRIBUTED);
    store.save(&last_distributed)
}

pub fn read_last_distributed(storage: &dyn Storage) -> StdResult<u64> {
    singleton_read(storage, KEY_LAST_DISTRIBUTED).load()
}

pub fn store_weight(storage: &mut dyn Storage, asset_token: &String, weight: u32) -> StdResult<()> {
    let mut weight_bucket: Bucket<u32> = Bucket::new(storage, PREFIX_WEIGHT);
    weight_bucket.save(asset_token.as_str().as_bytes(), &weight)
}

pub fn read_weight(storage: &dyn Storage, asset_token: &String) -> StdResult<u32> {
    let weight_bucket: ReadonlyBucket<u32> = ReadonlyBucket::new(storage, PREFIX_WEIGHT);
    match weight_bucket.load(asset_token.as_str().as_bytes()) {
        Ok(v) => Ok(v),
        _ => Err(StdError::generic_err("No distribution info stored")),
    }
}

pub fn remove_weight(storage: &mut dyn Storage, asset_token: &String) {
    let mut weight_bucket: Bucket<u32> = Bucket::new(storage, PREFIX_WEIGHT);
    weight_bucket.remove(asset_token.as_str().as_bytes());
}

pub fn read_all_weight(storage: &dyn Storage) -> StdResult<Vec<(String, u32)>> {
    let weight_bucket: ReadonlyBucket<u32> = ReadonlyBucket::new(storage, PREFIX_WEIGHT);
    weight_bucket
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (k, v) = item?;

            Ok((
                (unsafe { std::str::from_utf8_unchecked(&k) }).to_string(),
                v,
            ))
        })
        .collect()
}
