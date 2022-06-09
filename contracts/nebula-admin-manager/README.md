# Nebula Admin Manager

The admin manager contract is responsible for executing contract migrations and admin role transfers. All operations in the admin manager contract can only be executed by creating and executing a poll on the Nebula Governance.

- [Nebula Admin Manager](#nebula-admin-manager)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [UpdateOwner](#updateowner)
    - [ExecuteMigrations](#executemigrations)
    - [AuthorizeClaim](#authorizeclaim)
    - [ClaimAdmin](#claimadmin)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [MigrationRecords](#migrationrecords)
    - [AuthRecords](#authrecords)

## InstantiateMsg

The instantiate message takes the address of the `owner` account that will be the owner of the contract and `admin_claim_period`, the duration of admin privilege delegation in blocks.

```json
{
  "owner": String,
  "admin_claim_period": u64
}
```

## ExecuteMsg

### UpdateOwner

Updates the owner address of the admin manager contract

```json
{
  "update_owner": {
    "owner": String
  }
}
```

- `owner`: address of the new owner of the `admin manager` contract

### ExecuteMigrations

Migrates the specified contracts to the new code_ids

```json
{
  "execute_migrations": {
    "migrations": Vec<(String, u64, Binary)>
  }
}
```

- `migrations`: The migrations detail

The vector struct is:

- `contract_address` (String): address of the contract to be migrated
- `code_id` (u64): code_id of the new contract
- `migrate_msg` (Binary): MigrateMsg in binary

### AuthorizeClaim

Delegates admin privileges to migrate contracts to a specified address

```json
{
  "authorize_claim": {
    "authorized_addr": String
  }
}
```

- `authorized_addr`: address to temporarily delegate the admin privilege to

### ClaimAdmin

Claims the rights to the admin role

```json
{
  "claim_admin": {
    "contract": String
  }
}
```

- `contract`: address of contract that the user will have the rights to migrate

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
  "config": {}
}
```

### MigrationRecords

Returns the history of `execute_migrations` records

```json
{
  "migration_records": {
    "start_after": Option<u64>,
    "limit": Option<u32>
  }
}
```

- `start_after`: block height to return the migration history from
- `limit`: max number of migration records to return

### AuthRecords

Returns the history of `authorize_claim` transactions

```json
{
  "auth_records": {
    "start_after": Option<u64>,
    "limit": Option<u32>
  }
}
```

- `start_after`: block height to return the auth history from
- `limit`: max number of auth records to return
