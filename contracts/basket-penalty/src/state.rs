use cosmwasm_std::{Decimal, StdResult, Storage};
use cosmwasm_storage::{bucket, bucket_read};

/// prices: Map<asset:
pub static PREFIX_PRICES: &[u8] = b"prices";

pub fn read_price<S: Storage>(storage: &S, asset: &String) -> StdResult<Decimal> {
    bucket_read(PREFIX_PRICES, storage).load(asset.as_bytes())
}

pub fn set_price<S: Storage>(storage: &mut S, asset: &String, price: &Decimal) -> StdResult<()> {
    bucket(PREFIX_PRICES, storage).save(asset.as_bytes(), price)
}
