use cosmwasm_std::{Addr, StdResult, Storage};
use cosmwasm_storage::{singleton, singleton_read};

/// owner: Addr
pub static PREFIX_OWNER: &[u8] = b"owner";
/// neb: Addr
pub static PREFIX_NEB: &[u8] = b"neb";

//////////////////////////////////////////////////////////////////////
/// OWNER
//////////////////////////////////////////////////////////////////////

pub fn read_owner(storage: &dyn Storage) -> StdResult<Addr> {
    singleton_read(storage, PREFIX_OWNER).load()
}

pub fn set_owner(storage: &mut dyn Storage, owner: &Addr) -> StdResult<()> {
    singleton(storage, PREFIX_OWNER).save(owner)
}

//////////////////////////////////////////////////////////////////////
/// NEBULA TOKEN
//////////////////////////////////////////////////////////////////////

pub fn read_neb(storage: &dyn Storage) -> StdResult<Addr> {
    singleton_read(storage, PREFIX_NEB).load()
}

pub fn set_neb(storage: &mut dyn Storage, owner: &Addr) -> StdResult<()> {
    singleton(storage, PREFIX_NEB).save(owner)
}
