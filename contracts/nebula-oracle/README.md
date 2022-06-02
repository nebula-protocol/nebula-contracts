# Nebula Oracle

The Oracle Contract exposes an interface for accessing the latest reported price for assets used by the Nebula Protocol.

Prices are only considered valid for 60 seconds. If no new prices are published after the data has expired, the Nebula Protocol will disable all mint and burn operations until the price feed resumes

## InstantiateMsg

```json
{
    "owner": String,
    "oracle_addr": String,
    "base_denom": String
}
```

- `owner`: address of the owner of the `oracle` contract
- `oracle_addr`: address of the [TeFi Oracle Hub](https://github.com/terra-money/tefi-oracle-contracts/tree/main/contracts/oracle-hub) contract
- `base_denom` base denom when calculating prices (`TODO` in real cases)

## ExecuteMsg

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": Option<String>,
        "oracle_addr": Option<String>,
        "base_denom": Option<String>
    }
}
```

- `owner`: address of the new owner of the `oracle` contract
- `oracle_addr`: address of the new [TeFi Oracle Hub](https://github.com/terra-money/tefi-oracle-contracts/tree/main/contracts/oracle-hub) contract
- `base_denom` new base denom when calculating prices

## QueryMsg

### Config

Returns general contract parameters

```json
{
    "config": {}
}
```

### Price

Returns the latest price by calculating `latest_base_price`/`latest_quote_price`

```json
{
    "price": {
        "base_asset": AssetInfo,
        "quote_asset": AssetInfo
    }
}
```

- `base_asset`: base asset to calculate the latest price
- `quote_asset`: quote asset to calculate the latest price

