# Nebula Airdrop

The airdrop contract stores airdrop information and provides the ability to initiate airdrop claims for Nebula's LUNA staker airdrop on protocol genesis. Admin users will submit a claim for an airdrop with the associated Merkle proof. Users can then submit their portion of the proof to claim their airdrops

- [Nebula Airdrop](#nebula-airdrop)
  - [InstantiateMsg](#instantiatemsg)
  - [ExecuteMsg](#executemsg)
    - [`UpdateConfig`](#updateconfig)
    - [`RegisterMerkleRoot`](#registermerkleroot)
    - [`Claim`](#claim)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [MerkleRoot](#merkleroot)
    - [LatestStage](#lateststage)
    - [IsClaimed](#isclaimed)

## InstantiateMsg

The instantiate message takes the address of the `owner` account that will be submitting the Merkle poof and `nebula_token`, the address of the Nebula governance token to be distributing as rewards.

```json
{
    "owner": String,
    "nebula_token": String
}
```

- `owner`: address of the owner of the `airdrop` contract
- `nebula_token`: contract address of Nebula Token (NEB)

## ExecuteMsg

### `UpdateConfig`

Updates contract variables, namely the `owner` of the contract and the address of the Nebula token contract.

```json
{
    "update_config": {
        "owner": String,
        "nebula_token": String
    }
}
```

- `owner`: address of the new owner of the `airdrop` contract
- `nebula_token`: contract address of new Nebula Token (NEB)

### `RegisterMerkleRoot`

Registers a new Merkle root under a next stage.

```json
{
    "register_merkle_root": {
        "merkle_root": String
    }
}
```

- `merkle_root`: the Merkle root to register and for the next stage of the airdrop

### `Claim`

Initiates an airdrop claim transaction for the message sender.

```json
{
    "claim": {
        "stage": u8,
        "amount": Uint128,
        "proof": Vec<String>
    }
}
```

- `stage`: stage of airdrop to be claimed
- `amount`: amount of the airdrop to be claimed by the sender at the specified stage
- `proof`: Merkle proof at the specified stage

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
    "config": {}
}
```

### MerkleRoot

Returns the Merkle root registered under the given stage

```json
{
    "merkle_root": {
        "stage": u8
    }
}
```

- `stage`: The stage number to query the Markle root of

### LatestStage

Returns the latest stage containing a Merkle root in the airdrop contract.

```json
{
    "latest_stage": {}
}
```

### IsClaimed

Returns whether the specified address already claimed their airdrop of the given stage.

```json
{
    "is_claimed": {
        "stage": u8,
        "address": String
    }
}
```

- `stage`: The stage number in which to query the claim status for
- `address`: The address of the user to query the airdrop claim status of
