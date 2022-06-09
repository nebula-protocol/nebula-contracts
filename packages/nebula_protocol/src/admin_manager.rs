use cosmwasm_std::Binary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// ## Description
/// This structure stores the basic settings for creating a new admin manager contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Owner of the admin manager contract
    pub owner: String,
    /// The duration of admin privilege delegation to a defined address when [`ExecuteMsg::AuthorizeClaim`] occurs.
    pub admin_claim_period: u64,
}

/// ## Description
/// This structure describes the execute messages of the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /////////////////////
    /// OWNER CALLABLE
    /////////////////////

    /// UpdateOwner updates the contract owner
    UpdateOwner {
        /// Address to claim the contract ownership
        owner: String,
    },
    /// ExecuteMigrations migrates the specified contracts
    ExecuteMigrations {
        /// List of migrations to be executed
        /// (address of the current contract to be migrated, code_id of the new contract, MigrateMsg)
        migrations: Vec<(String, u64, Binary)>,
    },
    /// AuthorizeClaim delegates admin privileges to migrate contracts to a specified address
    AuthorizeClaim {
        /// Address to temporarily delegate the admin privilege to
        authorized_addr: String,
    },

    /////////////////////
    /// USER CALLABLE
    /////////////////////

    /// ClaimAdmin claims the rights to the admin keys
    ClaimAdmin {
        /// Address of contract that the user will have the rights to migrate
        contract: String,
    },
}

/// ## Description
/// This structure describes the available query messages for the admin manager contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Config returns contract settings specified in the custom [`ConfigResponse`] structure.
    Config {},
    /// MigrationRecords returns the history of [`ExecuteMsg::ExecuteMigrations`] records.
    MigrationRecords {
        /// Optional block height to return the migration history from
        start_after: Option<u64>,
        /// Optional max number of migration records to return
        limit: Option<u32>,
    },
    /// AuthRecords returns the history of [`ExecuteMsg::AuthorizeClaim`] transactions
    AuthRecords {
        /// Optional block height to return the auth history from
        start_after: Option<u64>,
        /// Optional max number of auth records to return
        limit: Option<u32>,
    },
}

/// ## Description
/// A custom struct for each query response that returns general contract settings/configs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Owner of the admin manager contract
    pub owner: String,
    /// The duration of admin privilege delegation to a defined address when
    /// [`ExecuteMsg::AuthorizeClaim`] occurs.
    pub admin_claim_period: u64,
}

/// ## Description
/// A custom struct for an address and its related authorization block height.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthRecordResponse {
    /// Address of an authorized account
    pub address: String,
    /// Start block height of the authorized period
    pub start_height: u64,
    /// End block height of the authorized period
    pub end_height: u64,
}

/// ## Description
/// A custom struct for each query response that returns a list of addresses authorization
/// in a custom [`AuthRecordResponse`] structure.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthRecordsResponse {
    /// A list of an address and its related authorization block height
    pub records: Vec<AuthRecordResponse>,
}

/// ## Description
/// A custom struct for each migration details.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationItem {
    /// Address of the migrated contract
    pub contract: String,
    /// New code_id for the migration
    pub new_code_id: u64,
    /// MigrateMsg for the migration
    pub msg: Binary,
}

/// ## Description
/// A custom struct for each [`ExecuteMsg::ExecuteMigrations`] execution.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationRecordResponse {
    /// Address of an executor
    pub executor: String,
    /// Block height of the execution
    pub height: u64,
    /// A list of migrations and their details in the execution
    pub migrations: Vec<MigrationItem>,
}

/// ## Description
/// A custom struct for each query response that returns a list of migration executions.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrationRecordsResponse {
    /// A list of migration executions
    pub records: Vec<MigrationRecordResponse>,
}

/// ## Description
/// A struct used for migrating contracts.
/// Currently take no arguments for migrations.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
