# Nebula Incentives Custody

- [Nebula Incentives Custody](#nebula-incentives-custody)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [RequestNeb](#requestneb)
    - [UpdateOwner](#updateowner)

## InstantiateMsg

```json
{
    "owner": String,
    "nebula_token": String
}
```

## ExecuteMsg

### RequestNeb

Transfer NEB tokens of `amount` from the incentives custody contract to the caller. Only callable by the custody's owner (the [`incentives`](../nebula-incentives/) in real cases)


```json
{
    "request_neb": {
        "amount": Uint128
    }
}
```

- `amount`: amount of NEB tokens to transfer

### UpdateOwner

Update the owner of the custody contract

```json
{
    "update_owner": {
        "owner": String
    }
}
```

- `owner`: address of the new custody contract owner