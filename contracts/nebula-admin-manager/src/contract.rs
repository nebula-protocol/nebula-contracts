#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::state::{
    is_addr_authorized, AuthRecord, Config, MigrationRecord, AUTH_LIST, AUTH_RECORDS_BY_HEIGHT,
    CONFIG, MIGRATION_RECORDS_BY_HEIGHT,
};

use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use nebula_protocol::admin_manager::{
    AuthRecordResponse, AuthRecordsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg,
    MigrateMsg, MigrationItem, MigrationRecordResponse, MigrationRecordsResponse, QueryMsg,
};

/// Contract name that is used for migration.
const CONTRACT_NAME: &str = "nebula-admin-manager";
/// Contract version that is used for migration.
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(msg.owner.as_str())?,
        admin_claim_period: msg.admin_claim_period,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => try_update_owner(deps, info, owner),
        ExecuteMsg::ExecuteMigrations { migrations } => {
            try_execute_migrations(deps, info, env, migrations)
        }
        ExecuteMsg::AuthorizeClaim { authorized_addr } => {
            try_authorize_claim(deps, info, env, authorized_addr)
        }
        ExecuteMsg::ClaimAdmin { contract } => try_claim_admin(deps, info, env, contract),
    }
}

pub fn try_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let validated_new_owner = deps.api.addr_validate(owner.as_str())?;

    config.owner = validated_new_owner;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_owner"),
        attr("new_owner", owner),
    ]))
}

pub fn try_execute_migrations(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    migrations: Vec<(String, u64, Binary)>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut migration_msgs: Vec<CosmosMsg> = vec![];
    let mut migrations_raw: Vec<(Addr, u64, Binary)> = vec![];

    for migration in migrations.iter() {
        let validated_contract_addr = deps.api.addr_validate(migration.0.as_str())?;

        migration_msgs.push(CosmosMsg::Wasm(WasmMsg::Migrate {
            contract_addr: validated_contract_addr.to_string(),
            new_code_id: migration.1,
            msg: migration.2.clone(),
        }));

        migrations_raw.push((validated_contract_addr, migration.1, migration.2.clone()));
    }

    let number_of_migrations = migration_msgs.len();
    let current_block_height = env.block.height;
    let migration_record = MigrationRecord {
        executor: info.sender,
        height: current_block_height,
        migrations: migrations_raw,
    };
    MIGRATION_RECORDS_BY_HEIGHT.save(deps.storage, current_block_height, &migration_record)?;

    Ok(Response::new()
        .add_messages(migration_msgs)
        .add_attributes(vec![
            attr("action", "execute_migrations"),
            attr("number_of_migrations", number_of_migrations.to_string()),
        ]))
}

pub fn try_authorize_claim(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    authorized_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    let validated_authorized_addr = deps.api.addr_validate(authorized_addr.as_str())?;
    let claim_start = env.block.height;
    let claim_end = claim_start + config.admin_claim_period;

    let record = AuthRecord {
        address: validated_authorized_addr.clone(),
        start_height: claim_start,
        end_height: claim_end,
    };
    AUTH_LIST.save(deps.storage, validated_authorized_addr, &claim_end)?;
    AUTH_RECORDS_BY_HEIGHT.save(deps.storage, claim_start, &record)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "authorize_claim"),
        attr("authorized_address", authorized_addr),
        attr("claim_start", claim_start.to_string()),
        attr("claim_end", claim_end.to_string()),
    ]))
}

pub fn try_claim_admin(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    contract: String,
) -> Result<Response, ContractError> {
    let validated_contract = deps.api.addr_validate(contract.as_str())?;

    if !is_addr_authorized(deps.storage, info.sender.clone(), env.block.height) {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
            contract_addr: validated_contract.to_string(),
            admin: info.sender.to_string(),
        }))
        .add_attributes(vec![
            attr("action", "claim_admin"),
            attr("contract", contract),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AuthRecords { start_after, limit } => {
            to_binary(&query_auth_records(deps, start_after, limit)?)
        }
        QueryMsg::MigrationRecords { start_after, limit } => {
            to_binary(&query_migration_records(deps, start_after, limit)?)
        }
    };

    res.map_err(|err| err.into())
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: config.owner.to_string(),
        admin_claim_period: config.admin_claim_period,
    })
}

pub fn query_auth_records(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<AuthRecordsResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_after.map(Bound::exclusive);

    let records = AUTH_RECORDS_BY_HEIGHT
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item| {
            let (_, record) = item?;

            Ok(AuthRecordResponse {
                address: record.address.to_string(),
                start_height: record.start_height,
                end_height: record.end_height,
            })
        })
        .collect::<Result<Vec<AuthRecordResponse>, ContractError>>()?;

    Ok(AuthRecordsResponse { records })
}

pub fn query_migration_records(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> Result<MigrationRecordsResponse, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let end = start_after.map(Bound::exclusive);

    let records = MIGRATION_RECORDS_BY_HEIGHT
        .range(deps.storage, None, end, Order::Descending)
        .take(limit)
        .map(|item| {
            let (_, record) = item?;
            let migration_items = record
                .migrations
                .iter()
                .map(|item| {
                    let res = MigrationItem {
                        contract: item.0.to_string(),
                        new_code_id: item.1,
                        msg: item.2.clone(),
                    };
                    Ok(res)
                })
                .collect::<Result<Vec<MigrationItem>, ContractError>>()?;

            Ok(MigrationRecordResponse {
                executor: record.executor.to_string(),
                height: record.height,
                migrations: migration_items,
            })
        })
        .collect::<Result<Vec<MigrationRecordResponse>, ContractError>>()?;

    Ok(MigrationRecordsResponse { records })
}

/// ## Description
/// Exposes the migrate functionality in the contract.
///
/// ## Params
/// - **_deps** is an object of type [`DepsMut`].
///
/// - **_env** is an object of type [`Env`].
///
/// - **_msg** is an object of type [`MigrateMsg`].
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}
