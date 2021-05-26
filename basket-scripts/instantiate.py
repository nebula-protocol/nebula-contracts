from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 2
DEFAULT_EFFECTIVE_DELAY = 2
DEFAULT_EXPIRATION_PERIOD = 20000
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"
DEFAULT_SNAPSHOT_PERIOD = 0
DEFAULT_VOTER_WEIGHT = "0.1"


def instantiate_terraswap_factory_contract(terraswap_factory_code_id, pair_code_id, token_code_id):
    print(f"[deploy] - instantiate terraswap factory contract")
    terraswap_factory_contract = instantiate_contract(
        terraswap_factory_code_id,
        {"pair_code_id": int(pair_code_id), "token_code_id": int(token_code_id)},
        seq(),
    )
    return terraswap_factory_contract

def instantiate_factory_contract(factory_code_id, token_code_id, basket_code_id):
    print(f"[deploy] - instantiate factory")
    factory_contract = instantiate_contract(
        factory_code_id,
        {
            "token_code_id": int(token_code_id),
            "cluster_code_id": int(basket_code_id),
            "base_denom": "uusd",
            "protocol_fee_rate": "0.001",
            # rewards for lp stakers
            "distribution_schedule": [[0, 100000, "1000000"]],
        },
        seq(),
    )
    return factory_contract


def instantiate_nebula_token(token_code_id, factory_contract):
    print(f"[deploy] - instantiate nebula token")
    nebula_token = instantiate_contract(
        token_code_id,
        {
            "name": "Nebula Token",
            "symbol": "NEB",
            "decimals": 6,
            "initial_balances": [
                {
                    "address": deployer.key.acc_address,
                    "amount": "100000000000000",
                },
                {
                    "address": factory_contract,
                    "amount": "10000000000",
                },
            ],
            # maybe ?
            "minter": {"minter": factory_contract, "cap": None},
        },
        seq(),
    )
    return nebula_token

def instantiate_staking_contract(staking_code_id, factory_contract, nebula_token, terraswap_factory_contract):
    print(f"[deploy] - create staking contract")
    staking_contract = instantiate_contract(
        staking_code_id,
        {
            "owner": factory_contract,
            "nebula_token": nebula_token,
            "terraswap_factory": terraswap_factory_contract,
            "base_denom": "uusd",
            "premium_min_update_interval": 5,
        },
        seq(),
    )
    return staking_contract

def instantiate_gov_contract(gov_code_id, nebula_token):
    print(f"[deploy] - instantiate nebula governance")
    gov_contract = instantiate_contract(
        gov_code_id,
        {
            "nebula_token": nebula_token,
            "quorum": DEFAULT_QUORUM,
            "threshold": DEFAULT_THRESHOLD,
            "voting_period": DEFAULT_VOTING_PERIOD,
            "effective_delay": DEFAULT_EFFECTIVE_DELAY,
            "expiration_period": DEFAULT_EXPIRATION_PERIOD,
            "proposal_deposit": DEFAULT_PROPOSAL_DEPOSIT,
            "voter_weight": DEFAULT_VOTER_WEIGHT,
            "snapshot_period": DEFAULT_SNAPSHOT_PERIOD,
        },
        seq(),
    )
    return gov_contract

def instantiate_collector_contract(collector_code_id, gov_contract, terraswap_factory_contract, nebula_token, factory_contract):
    print(f"[deploy] - instantiate nebula collector")
    collector_contract = instantiate_contract(
        collector_code_id,
        {
            "distribution_contract": gov_contract,
            "terraswap_factory": terraswap_factory_contract,
            "nebula_token": nebula_token,
            "base_denom": "uusd",
            "owner": factory_contract
        },
        seq(),
    )
    return collector_contract

def instantiate_penalty_contract(penalty_code_id, factory_contract):
    print(f"[deploy] - instantiate penalty contract")
    penalty_contract = instantiate_contract(
        penalty_code_id,
        {
            "penalty_params": {
                "penalty_amt_lo": "0.1",
                "penalty_cutoff_lo": "0.01",
                "penalty_amt_hi": "0.5",
                "penalty_cutoff_hi": "0.1",
                "reward_amt": "0.05",
                "reward_cutoff": "0.02",
            },
            "owner": factory_contract
        },
        seq(),
    )
    return penalty_contract

def instantiate_wbtc_contract(token_code_id):
    print(f"[deploy] - instantiate wBTC")
    wBTC = instantiate_contract(
        token_code_id,
        {
            "name": "Wrapped Bitcoin",
            "symbol": "wBTC",
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": "400000000"}
            ],
            "mint": None,
        },
        seq(),
    )
    return wBTC

def instantiate_weth_contract(token_code_id):
    print(f"[deploy] - instantiate wETH")
    wETH = instantiate_contract(
        token_code_id,
        {
            "name": "Wrapped Ethereum",
            "symbol": "wETH",
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": "20000000000"}
            ],
            "mint": None,
        },
        seq(),
    )
    return wETH

def instantiate_oracle_contract(oracle_code_id):
    print(f"[deploy] - instantiate oracle")
    oracle = instantiate_contract(oracle_code_id, {}, seq())
    return oracle

def instantiate_community_contract(community_id, gov_contract, nebula_token):
    # instantiate community pool
    print(f"[deploy] - instantiate community pool")
    nebula_community = instantiate_contract(
        community_id,
        {
            "owner": gov_contract,
            "nebula_token": nebula_token,
            "spend_limit": "1000000"
        },
        seq(),
    )
    return nebula_community

def instantiate_airdrop_contract(airdrop_id, nebula_token):
    # instantiate airdrop contract
    print(f"[deploy] - instantiate airdrop contract")
    airdrop = instantiate_contract(
        airdrop_id,
        {
            "owner": deployer.key.acc_address,
            "nebula_token": nebula_token,
        },
        seq(),
    )
    return airdrop

def instantiate_incentives_contract(incentives_id, factory_contract, terraswap_factory, nebula_token):
    print(f"[deploy] - instantiate incentives contract")
    incentives = instantiate_contract(
        incentives_id,
        {
            "owner": deployer.key.acc_address,
            "factory": factory_contract,
            "terraswap_factory": terraswap_factory,
            "nebula_token": nebula_token,
            "base_denom": "uusd"
        },
        seq()
    )
    return incentives
