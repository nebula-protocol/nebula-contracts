use cosmwasm_std::{Decimal, StdResult, Storage, HumanAddr};
use cosmwasm_storage::{singleton, singleton_read};

/// prices: Map<asset:
pub static PREFIX_OWNER: &[u8] = b"owner";
pub static PREFIX_NEB: &[u8] = b"neb";

pub fn read_owner<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    singleton_read(storage, PREFIX_OWNER).load()
}

pub fn set_owner<S: Storage>(storage: &mut S, owner: &HumanAddr) -> StdResult<()> {
    singleton(storage, PREFIX_OWNER).save(owner)
}

pub fn read_neb<S: Storage>(storage: &S) -> StdResult<HumanAddr> {
    singleton_read(storage, PREFIX_NEB).load()
}

pub fn set_neb<S: Storage>(storage: &mut S, owner: &HumanAddr) -> StdResult<()> {
    singleton(storage, PREFIX_NEB).save(owner)
}
