# Nebula Governance

The Gov Contract contains logic for holding polls and Nebula Token (NEB) staking, and allows the Nebula Protocol to be governed by its users in a decentralized manner. After the initial bootstrapping of Nebula Protocol contracts, the Gov Contract is assigned to be the owner of itself and other contracts.

New proposals for change are submitted as polls, and are voted on by NEB stakers through the voting procedure. Polls can contain messages that can be executed directly without changing the NEB Protocol code.

The Gov Contract keeps a balance of NEB tokens, which it uses to reward stakers with funds it receives from trading fees sent by the Nebula Collector and user deposits from creating new governance polls. This balance is separate from the Community Pool, which is held by the Community contract (owned by the Gov contract).

- [Nebula Governance](#nebula-governance)
  - [InstantitateMsg](#instantitatemsg)
  - [ExecuteMsg](#executemsg)
    - [Receive](#receive)
    - [UpdateConfig](#updateconfig)
    - [CastVote](#castvote)
    - [WithdrawVotingTokens](#withdrawvotingtokens)
    - [WithdrawVotingRewards](#withdrawvotingrewards)
    - [StakeVotingRewards](#stakevotingrewards)
    - [SnapshotPoll](#snapshotpoll)
    - [EndPoll](#endpoll)
    - [ExecutePoll](#executepoll)
  - [Receive Hook (CW20ReceiveMsg)](#receive-hook-cw20receivemsg)
    - [StakeVotingTokens](#stakevotingtokens)
    - [CreatePoll](#createpoll)
    - [DepositReward](#depositreward)
  - [QueryMsg](#querymsg)
    - [Config](#config)
    - [State](#state)
    - [Staker](#staker)
    - [Poll](#poll)
    - [Polls](#polls)
    - [Voters](#voters)

## InstantitateMsg

```json
{
    "nebula_token": String,
    "quorum": Decimal,
    "threshold": Decimal,
    "voting_period": u64,
    "effective_delay": u64,
    "proposal_depsoit": U128,
    "voter_weight": Decimal,
    "snapshot_period": u64
}
```

- `nebula_token`: contract address of Nebula Token (NEB)
- `quorum`: minimum percentage of participation required for a poll to pass
- `threshold`: minimum percentage of yes votes required for a poll to pass
- `voting_period`: number of blocks during which votes can be cast
- `effective_delay`: number of blocks after a poll passes to apply changes
- `proposal_deposit`: minimum NEB deposit required for a new poll to be submitted
- `voter_weight`: ratio of protocol fee which will be distributed among the governance poll voters
- `snapshot_period`: minimum number of blocks before the end of voting period which snapshot could be taken to lock the current quorum for a poll

## ExecuteMsg

### Receive

Can be called during a CW20 token transfer when the Gov contract is the recipient. Allows the token transfer to execute a [Receive Hook](#receive-hook-cw20receivemsg) as a subsequent action within the same transaction.

```json
{
    "receive": {
        "amount": Uint128,
        "sender": String,
        "msg": Option<Binary>
    }
}
```

- `amount`: amount of tokens received
- `sender`: sender of the token transfer
- `msg`: Base64-encoded JSON of the Receive Hook

### UpdateConfig

Updates contract variables

```json
{
    "update_config": {
        "owner": Option<String>,
        "quorum": Option<Decimal>,
        "threshold": Option<Decimal>,
        "voting_period": Option<u64>,
        "effective_delay": Option<u64>,
        "proposal_deposit": Option<Uint128>,
        "voter_weight": Option<Decimal>,
        "snapshot_period": Option<u64>
    }
}
```

- `owner`: address of the owner of the `gov` contract
- `quorum`: Minimum percentage of participation required for a poll to pass
- `threshold`: Minimum percentage of yes votes required for a poll to pass
- `voting_period`: Number of blocks during which votes can be cast
- `effective_delay`: Number of blocks after a poll passes to apply changes
- `proposal_deposit`: Minimum NEB deposit required for a new poll to be submitted
- `voter_weight`: Ratio of protocol fee which will be distributed among the governance poll voters
- `snapshot_period`: Minimum number of blocks before the end of voting period which snapshot could be taken to lock the current quorum for a poll

### CastVote

Submits a user's vote for an active poll. Once a user has voted, they cannot change their vote with subsequent messages (increasing voting power, changing vote option, cancelling vote, etc.)

```json
{
  "cast_vote": {
    "poll_id": u64,
    "vote": VoteOption,
    "amount": Uint128
  }
}
```

- `poll_id`: Poll ID
- `vote`: Can be `yes`,`no`, or `abstain`
- `amount`: Amount of voting power (staked NEB) to allocate

### WithdrawVotingTokens

Removes deposited NEB tokens from a staking position and returns them to a user's balance.

```json
{
  "withdraw_voting_tokens": {
    "amount": Option<Uint128>
  }
}
```

- `amount`: Amount of NEB tokens to withdraw. If empty, all staked NEB tokens are withdrawn

### WithdrawVotingRewards

Withdraws a user’s voting reward for user’s voted governance poll after end_poll has happened.

```json
{
  "withdraw_voting_rewards": {}
}
```

### StakeVotingRewards

Immediately re-stakes user's voting rewards to Gov Contract.

```json
{
  "stake_voting_rewards": {}
}
```

### SnapshotPoll

Snapshot of poll’s current `quorum` status is saved when the block height enters `snapshot_period`.

```json
{
  "snapshot_poll": {
    "poll_id": u64
  }
}
```

- `poll_id`: Poll ID

### EndPoll

Can be issued by anyone to end the voting for an active poll. Triggers tally the results to determine whether the poll has passed. The current block height must exceed the end height of voting phase.

```json
{
  "end_poll": {
    "poll_id": u64
  }
}
```

- `poll_id`: Poll ID

### ExecutePoll

Can be issued by anyone to implement into action the contents of a passed poll. The current block height must exceed the end height of the poll's effective delay.

```json
{
  "execute_poll": {
    "poll_id": u64
  }
}
```

- `poll_id`: Poll ID

## Receive Hook (CW20ReceiveMsg)

**WARNING: If you send NEB tokens to the Gov contract without issuing this hook, they will not be staked and will be irrevocably donated to the reward pool for stakers.**
### StakeVotingTokens

Issued when sending NEB tokens to the Gov contract to add them to their NEB staking position.

```json
{
  "stake_voting_tokens": {}
}
```

### CreatePoll

Issued when sending NEB tokens to the Gov contract to create a new poll. Will only succeed if the amount of tokens sent meets the configured `proposal_deposit` amount. Contains a generic message to be issued by the Gov contract if it passes (can invoke messages in other contracts it owns).

```json
{
  "create_poll": {
    "title": String,
    "description": String,
    "link": Option<String>,
    "execute_data": Option<Vec<ExecuteData>>
  }
}
```

- `title`: Poll title,
- `description`: Poll description
- `link`: URL to external post about poll (forum, PDF, etc.)
- `execute_data`: Messages to be executed by Gov contract

The `ExecuteData` type then has the following structure:

```json
{
  "execute_data": {
    "contract": Addr,
    "msg": Binary
  }
}
```

### DepositReward

Reward is distributed between NEB stakers and governance poll voters based on `voter_weight` when rewards are sent from the [`collector`]((../nebula-collector/)) contract.

```json
{
  "deposit_reward": {}
}
```

## QueryMsg

### Config

Returns the current configuration information for the contract

```json
{
  "config": {}
}
```

### State

Returns the current state of the contract

```json
{
  "state": {}
}
```

### Staker

Returns information on a staker

```json
{
  "staker": {
    "address": String
  }
}
```

- `address`: Address of staker

### Poll

Returns information on a single poll

```json
{
  "poll": {
    "poll_id": u64
  }
}
```

- `poll_id`: Poll ID

### Polls

Returns information on all of the polls created through the contract

```json
{
  "polls": {
    "filter": Option<PollStatus>,
    "limit": Option<u32>,
    "start_after": Option<u64>
  }
}
```

- `filter`: filter returned list of poll by status (can be `in_progress`, `passed`, `rejected`, `executed`, `expired`, or `failed`)
- `limit`: limit number of matching polls to fetch and return
- `start_after`: Begins search query at a specific ID

### Voters

Returns list of voter information for a given poll

```json
{
  "voters": {
    "limit": Option<u32>,
    "poll_id": u64,
    "start_after": Option<String>
  }
}
```

- `limit`: limit number of matching polls to fetch and return
- `poll_id`: Poll ID
- `start_after`: Begins search query with prefix