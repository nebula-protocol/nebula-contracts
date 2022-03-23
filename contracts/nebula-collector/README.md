# Nebula Collector

The Collector accumulates Nebula protocol fees and swaps them to NEB through the NEB <> UST Aastroport pair. Swapped NEB tokens are distributed to NEB stakers (sent to the [`gov](../nebula-gov/)) contract).

- [Nebula Collector](#nebula-collector)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [Convert](#convert)
    - [Distribute](#distribute)
    - [UpdateConfig](#updateconfig)
  - [QueryMsg](#querymsg)
    - [Config](#config)

## InstantiateMsg

```json
{
    "distribution_contract": String,
    "astroport_factory": String,
    "nebula_token": String,
    "base_denom": String,
    "owner": String
}
```

- `distribution_contract`: address at which to distribute the protocol fee rewards to (the [`gov`](../nebula-gov/) contract usually)
- `astroport_factory`: address of the [Astroport](https://astroport.fi) [factory](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract
- `nebula_token`: contract address of Nebula Token (NEB)
- `base_denom`: base denom for all swap operations (UST)
- `owner`: address of the owner of the `collector` contract

## ExecuteMsg

### Convert

Converts/swap either:

- Cluster tokens accrued through protocol fees to UST
- UST accrued from converting the cluster tokens to NEB

```json
{
    "convert": {
        "asset_token": String
    }
}
```

- `asset_token`: address of CW20 token to convert

### Distribute

Distributes the converted NEB tokens to the `distribution_contract`

```json
{
    "distribute": {},
}
```

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "distribution_contract": Option<String>,
        "astroport_factory": Option<String>,
        "nebula_token": Option<String>,
        "base_denom": Option<String>,
        "owner": Option<String>
    }
}
```

- `distribution_contract`: new distribution contract address
- `astroport_factory`: address of the new [Astroport](https://astroport.fi) [factory](https://github.com/astroport-fi/astroport-core/tree/main/contracts/factory) contract
- `nebula_token`: contract address of new Nebula Token (NEB)
- `base_denom`: new base denom
- `owner`: address of the new owner of the `cluster` contract

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
    "config": {}
}
```
