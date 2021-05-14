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

from basket import Oracle, Basket, CW20, Asset, Governance, Community, Airdrop

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 3
DEFAULT_EFFECTIVE_DELAY = 3
DEFAULT_EXPIRATION_PERIOD = 20000
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"

lt = LocalTerra(gas_prices={"uluna": "0.15"})

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

    print(f"[deploy] - store airdrop")
    airdrop_id = store_contract("airdrop", seq())


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
                    "amount": "1000000000000",
                },
            ],
            # maybe ?
            "minter": {"minter": deployer.key.acc_address, "cap": None},
        },
        seq(),
    )

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


    # give airdrop contract funds - should be covered by genesis
    print(f"[deploy] - initial funds to airdrop contract")
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, nebula_token, CW20.transfer(airdrop, "10000000")
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)
    print(result.logs[0].events_by_type)

    stage = 1
    claim_amount = "1000000"
    proof = [
        "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42",
        "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f",
        "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3",
        "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2"
    ]

    print(f"[deploy] - register merkle root")
    resp = execute_contract(
        deployer,
        airdrop,
        Airdrop.register_merkle_root("3b5c044802c4b768492f98fbd5a9253ed3dd97f5ff129de79a179249e2021766"),
        seq(),
    )
    print(resp.logs[0].events_by_type)

    print(f"[deploy] - claim airdrop")
    resp = execute_contract(
        deployer,
        airdrop,
        Airdrop.claim(stage, claim_amount, proof),
        seq(),
    )
    print(resp.logs[0].events_by_type)

    # Register
    # {
    #     "register_merkle_root": {
    #     "merkle_root": "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
    #     }
    # }

    # Claim with account terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8
    # {
    #     "claim": {
    #         "stage": 1,
    #         "amount": "1000000",
    #         "proof": [
    #             "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42",
    #             "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f",
    #             "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3",
    #             "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2"
    #         ]
    #     }
    # }

    

deploy()