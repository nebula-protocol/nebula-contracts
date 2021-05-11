"""Test governance deploy script.

NOTE: Normally, we can use fee estimation in Tequila, as well as rely on Wallet to auto
fetch the sequence number from the blockchain. Here, we have manual options for sequence
number and fee.

Why manually incrementing sequence number: tequila endpoint is load-balanced so in successive
transactions, the nodes may not have had time to catch up to each other, which may result
in a signature (chain id, account, sequence) mismatch.

Why manually setting fee: tequila node allows simulating (auto-estimating) fee up to
3000000 gas. Some transactions such as code uploads and burning basket token (which
incurs multiple CW20 transfers to the user may require more gas than permitted by the
fee estimation feature).
"""

import time
from terra_sdk.client.lcd import LCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

from basket import Oracle, Basket, CW20, Asset, Governance, Community

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 4
DEFAULT_EFFECTIVE_DELAY = 2
DEFAULT_EXPIRATION_PERIOD = 20000
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"

lt = LocalTerra(gas_prices = {
        "uluna": "0.15"
    })

if USE_LOCALTERRA:
    terra = lt
    deployer = lt.wallets["test1"]
else:
    gas_prices = {
        "uluna": "0.15",
        "usdr": "0.1018",
        "uusd": "0.15",
        "ukrw": "178.05",
        "umnt": "431.6259",
        "ueur": "0.125",
        "ucny": "0.97",
        "ujpy": "16",
        "ugbp": "0.11",
        "uinr": "11",
        "ucad": "0.19",
        "uchf": "0.13",
        "uaud": "0.19",
        "usgd": "0.2",
    }

    terra = LCDClient(
        "https://tequila-fcd.terra.dev", "tequila-0004", gas_prices=gas_prices
    )
    deployer = terra.wallet(lt.wallets["test1"].key)


def store_contract(contract_name, sequence):
    contract_bytes = read_file_as_b64(f"../artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)
    store_code_tx = deployer.create_and_sign_tx(
        msgs=[store_code], fee=StdFee(5000000, "2000000uluna"), sequence=sequence
    )
    result = terra.tx.broadcast(store_code_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_code_id(result)


def instantiate_contract(code_id, init_msg, sequence):
    instantiate = MsgInstantiateContract(deployer.key.acc_address, code_id, init_msg)
    instantiate_tx = deployer.create_and_sign_tx(
        msgs=[instantiate], sequence=sequence, fee_denoms=["uluna"]
    )
    result = terra.tx.broadcast(instantiate_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_contract_address(result)


def execute_contract(wallet, contract_address, execute_msg, sequence, fee=None):
    execute = MsgExecuteContract(wallet.key.acc_address, contract_address, execute_msg)
    execute_tx = wallet.create_and_sign_tx(
        msgs=[execute], sequence=sequence, fee_denoms=["uluna"], fee=fee
    )
    result = terra.tx.broadcast(execute_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return result


def get_amount(value, price):
    """Gets Uint128 amount of coin in order to get total value, assuming price."""
    return str(int(value / float(price) * 1000000))


sequence = deployer.sequence()


def seq():
    """Increments global sequence."""
    global sequence
    sequence += 1
    return sequence - 1


def deploy():
    print(f"DEPLOYING WITH ACCCOUNT: {deployer.key.acc_address}")
    print(f"[deploy] - store terraswap_token")
    token_code_id = store_contract("terraswap_token", seq())

    print(f"[deploy] - store basket_dummy_oracle")
    oracle_code_id = store_contract("basket_dummy_oracle", seq())

    print(f"[deploy] - store basket_contract")
    basket_code_id = store_contract("basket_contract", seq())

    print(f"[deploy] - store basket_gov")
    nebula_gov_code_id = store_contract("basket_gov", seq())

    print(f"[deploy] - store basket_community")
    community_id = store_contract("basket_community", seq())

    print(f"[deploy] - store penalty_contract")
    penalty_code_id = store_contract("basket_penalty", seq())

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
                "reward_cutoff": "0.02"
            }
        },
        seq()
    )

    # wrapped bitcoin
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

    # wrapped ether
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

    # instantiate oracle
    print(f"[deploy] - instantiate oracle")
    oracle = instantiate_contract(oracle_code_id, {}, seq())


    # instantiate nebula token
    print(f"[deploy] - instantiate nebula token")
    nebula_token = instantiate_contract(
        token_code_id,
        {
            "name": "Nebula Token",
            "symbol": "NEB",
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": "1000000000000"}
            ],
            "mint": None,
        },
        seq(),
    )

    # instantiate nebula governance contract
    print(f"[deploy] - instantiate nebula governance")
    nebula_gov = instantiate_contract(
        nebula_gov_code_id,
        {
            "nebula_token": nebula_token,
            "quorum": DEFAULT_QUORUM,
            "threshold": DEFAULT_THRESHOLD,
            "voting_period": DEFAULT_VOTING_PERIOD,
            "effective_delay": DEFAULT_EFFECTIVE_DELAY,
            "expiration_period": DEFAULT_EXPIRATION_PERIOD,
            "proposal_deposit": DEFAULT_PROPOSAL_DEPOSIT
        },
        seq(),
    )

    # instantiate community pool
    print(f"[deploy] - instantiate community pool")
    nebula_community = instantiate_contract(
        community_id,
        {
            "owner": nebula_gov,
            "nebula_token": nebula_token,
            "spend_limit": "1000000"
        },
        seq(),
    )

    # transfer some to community pool
    initial_community_neb_amt = "100000000"
    print(
        f"[deploy] - give community initial balance of nebula {initial_community_neb_amt}"
    )
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, nebula_token, CW20.transfer(nebula_community, initial_community_neb_amt)
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)
    print(result.logs[0].events_by_type)

    # instantiate basket
    print(f"[deploy] - instantiate basket")
    basket = instantiate_contract(
        basket_code_id,
        {
            "name": "Basket",
            "owner": deployer.key.acc_address,
            "assets": [Asset.cw20_asset_info(wBTC), Asset.cw20_asset_info(wETH)],
            "oracle": oracle,
            "penalty": penalty_contract,
            "target": [50, 50],
        },
        seq(),
    )

    # instantiate basket token
    print(f"[deploy] - instantiate basket token")
    basket_token = instantiate_contract(
        token_code_id,
        {
            "name": "Basket Token",
            "symbol": "BASKET",
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": "100000000"}
            ],
            "mint": {"minter": basket, "cap": None},
        },
        seq(),
    )

    # set basket token
    print(f"[deploy] - set basket token")
    execute_contract(deployer, basket, Basket.set_basket_token(basket_token), seq())

    execute_contract(deployer, basket, Basket.reset_owner(nebula_gov), seq())
    # set oracle prices
    print(f"[deploy] - set oracle prices")
    execute_contract(
        deployer,
        oracle,
        Oracle.set_prices(
            [
                [wBTC, "58000.0"],
                [wETH, "2800.0"]
            ]
        ),
        seq(),
    )

    # sets initial balance of basket contract
    amount = "1000000"

    print(
        f"[deploy] - give initial balances wBTC and wETH {amount}"
    )
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, wBTC, CW20.transfer(basket, amount)
            ),
            MsgExecuteContract(
                deployer.key.acc_address, wETH, CW20.transfer(basket, amount)
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)
    print(result.logs[0].events_by_type)

    # Create poll
    print(
        f"[deploy] - create poll to spend"
    )

    spend_amt = "10000"
    poll = Governance.create_poll(
        "Test", "Test", "TestLink1234",
        Governance.create_execute_msg(
            nebula_community,
            Community.spend(deployer.key.acc_address, spend_amt)
        )
    )

    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            nebula_gov, DEFAULT_PROPOSAL_DEPOSIT, poll
        ), 
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    # Stake
    print(
        f"[deploy] - stake 50% of basket tokens"
    )
    
    stake_amount = "500000000000"
    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            nebula_gov, stake_amount, Governance.stake_voting_tokens()
        ), 
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    # cast vote
    print(
        f"[deploy] - cast vote for YES"
    )
    
    result = execute_contract(
        deployer,
        nebula_gov,
        Governance.cast_vote(DEFAULT_POLL_ID, "yes", stake_amount), 
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )
    print(result.logs[0].events_by_type)

    # # increase block time (?)
    # global sequence
    # sequence += DEFAULT_EFFECTIVE_DELAY

    # execute poll
    print(f"sequence # is: {deployer.sequence()}")
    print(
        f"[deploy] - execute vote"
    )
    
    time.sleep(5)

    result = execute_contract(
        deployer,
        nebula_gov,
        Governance.end_poll(DEFAULT_POLL_ID),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    time.sleep(10)

    result = execute_contract(
        deployer,
        nebula_gov,
        Governance.execute_poll(DEFAULT_POLL_ID),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    res = result.logs[0].events_by_type
    assert res['from_contract']['recipient'][0] == deployer.key.acc_address
    assert res['from_contract']['amount'][0] == spend_amt


deploy()