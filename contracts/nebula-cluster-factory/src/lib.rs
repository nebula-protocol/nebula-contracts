pub mod contract;
pub mod math;
pub mod querier;
pub mod state;

mod response;

#[cfg(test)]
mod testing;

#[cfg(test)]
mod mock_querier;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::contract;
    use cosmwasm_std::{do_execute, do_init, do_query, ExternalApi, ExternalApi, ExternalStorage};

    #[no_mangle]
    extern "C" fn instantiate(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_init(
            &contract::init::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    #[no_mangle]
    extern "C" fn execute(env_ptr: u32, msg_ptr: u32) -> u32 {
        do_execute(
            &contract::execute::<ExternalStorage, ExternalApi, ExternalQuerier>,
            env_ptr,
            msg_ptr,
        )
    }

    #[no_mangle]
    extern "C" fn query(msg_ptr: u32) -> u32 {
        do_query(
            &contract::query::<ExternalStorage, ExternalApi, ExternalQuerier>,
            msg_ptr,
        )
    }

    // Other C externs like cosmwasm_vm_version_1, allocate, deallocate are available
    // automatically because we `use cosmwasm_std`.
}
