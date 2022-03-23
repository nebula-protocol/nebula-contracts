# Nebula Incentives Custody

- [Nebula Incentives Custody](#nebula-incentives-custody)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [RequestNeb](#requestneb)
    - [UpdateConfig](#updateconfig)

## InstantiateMsg

```json
{
    "owner": String,
    "nebula_token": String
}
```

- `owner`: address of the owner of the `incentives-custody` contract
- `nebula_token`: contract address of Nebula Token (NEB)

## ExecuteMsg

### RequestNeb

Transfers NEB tokens of `amount` from the incentives custody contract to the caller. Only callable by the custody's owner (the [`incentives`](../nebula-incentives/) in real cases)

```json
{
    "request_neb": {
        "amount": Uint128
    }
}
```

- `amount`: amount of NEB tokens to transfer

### UpdateConfig

Updates the owner of the custody contract

```json
{
    "update_config": {
        "owner": String
    }
}
```

- `owner`: address of the new custody contract owner