# Nebula Community

The Community Contract holds the funds of the Community Pool, which can be spent through a governance poll. 

## InstantiateMsg

```json
{
    "owner": String
}
```

- `owner`: address of the owner of the `community` contract

## ExecuteMsg

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": String
    }
}
```

- `owner`: address of the owner of the `community` contract

### Spend

Spends an amount of `asset` from the community pool and send it to `recipient`

```json
{
    "spend": {
        "asset" Asset,
        "recipient" String
    }
}
```

- `asset`: The asset and amount to send to `recipient`
- `recipient`: The address at which to send `recipient` to

### PassCommand

Executes the provided `wasm_msg`

```json
{
    "pass_command": {
        "wasm_msg": WasmMsg
    }
}
```

- `wasm_msg`: The `WasmMsg` to execute

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
    "config": {}
}
```