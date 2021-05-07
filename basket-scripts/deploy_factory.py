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

from basket import Oracle, Basket, CW20, Asset, Governance

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

DEFAULT_POLL_ID = 1
DEFAULT_QUORUM = "0.3"
DEFAULT_THRESHOLD = "0.5"
DEFAULT_VOTING_PERIOD = 4
DEFAULT_EFFECTIVE_DELAY = 6
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

    print(f"[deploy] - store basket_dummy_oracle")
    oracle_code_id = store_contract("basket_dummy_oracle", seq())

    print(f"[deploy] - store basket_contract")
    basket_code_id = store_contract("basket_contract", seq())

    print(f"[deploy] - store penalty_contract")
    penalty_code_id = store_contract("basket_penalty", seq())

    print(f"[deploy] - store terraswap_factory")
    terraswap_factory_code_id = store_contract("terraswap_factory", seq())

    print(f"[deploy] - store terraswap_pair")
    pair_code_id = store_contract("terraswap_pair", seq())

    print(f"[deploy] - store basket_staking")
    staking_code_id = store_contract("basket_staking", seq())

    print(f"[deploy] - store basket_factory")
    factory_code_id = store_contract("basket_factory", seq())

    print(f"[deploy] - instantiate terraswap factory contract")
    terraswap_factory_contract = instantiate_contract(
        terraswap_factory_code_id,
        {"pair_code_id": int(pair_code_id), "token_code_id": int(token_code_id)},
        seq(),
    )

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

    print(f"Instantiate factory")
    factory_contract = instantiate_contract(
        factory_code_id,
        {
            "token_code_id": int(token_code_id),
            "cluster_code_id": int(basket_code_id),
            "base_denom": "uusd",
            "distribution_schedule": [[1, 2, "3"]],
        },
        seq(),
    )

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

    print(f"[deploy] - post initialize factory")
    resp = execute_contract(
        deployer,
        factory_contract,
        {
            "post_initialize": {
                "owner": deployer.key.acc_address,
                "nebula_token": nebula_token,
                "oracle_contract": nebula_token,  # ??? provide arbitrary contract for now
                "terraswap_factory": terraswap_factory_contract,
                "staking_contract": staking_contract,
                "commission_collector": nebula_token,  # ??? provide arbitrary contract for now
            }
        },
        seq(),
    )

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
            }
        },
        seq(),
    )

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

    execute_contract(
        deployer,
        oracle,
        Oracle.set_prices(
            [
                [wBTC, "30000.0"],
                [wETH, "1500.0"],
                ["uluna", "15.00"],
            ]
        ),
        seq(),
    )

    resp = execute_contract(
        deployer,
        factory_contract,
        {
            "create_cluster": {
                "name": "BASKET",
                "symbol": "BSK",
                "params": {
                    "name": "BASKET",
                    "symbol": "BSK",
                    "penalty": penalty_contract,
                    "target": [50, 50],
                    "assets": [
                        Asset.cw20_asset_info(wBTC),
                        Asset.cw20_asset_info(wETH),
                    ],
                    "oracle": oracle,
                },
            }
        },
        seq(),
    )

    if resp.is_tx_error():
        raise Exception(resp.raw_log)
    print(basket_code_id)

    for event in resp.logs:
        import pprint

        pprint.pprint(event.events_by_type)

    logs = resp.logs[0].events_by_type

    instantiation_logs = logs["instantiate_contract"]
    code_ids = instantiation_logs["code_id"]
    addresses = instantiation_logs["contract_address"]
    assert code_ids[3] == basket_code_id
    basket = addresses[3]
    assert code_ids[2] == token_code_id
    basket_token = addresses[2]
    assert code_ids[1] == pair_code_id
    pair_contract = addresses[1]
    assert code_ids[0] == token_code_id
    lp_token = addresses[0]

    print("[deploy] - initialize basket with tokens via mint")
    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                wBTC,
                CW20.send(basket, "100", Basket.stage_asset()),
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                wETH,
                CW20.send(basket, "100", Basket.stage_asset()),
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                basket,
                Basket.mint(
                    [Asset.asset(wBTC, "100"), Asset.asset(wETH, "100")],
                    min_tokens="100",
                ),
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(stage_and_mint_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)

    debug_print = terra.wasm.contract_query(
        basket, {"basket_state": {"basket_contract_address": basket}}
    )
    print("Basket state after initialize --", debug_print)



    print(f"[deploy] - adding liquidity to pair contract")
    execute_contract(
        deployer,
        basket_token,
        {"increase_allowance": {"spender": pair_contract, "amount": "100"}},
        seq(),
    )

    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                pair_contract,
                {
                    "provide_liquidity": {
                        "assets": [
                            {
                                "info": {"token": {"contract_addr": basket_token}},
                                "amount": "100",
                            },
                            {
                                "info": {"native_token": {"denom": "uusd"}},
                                "amount": "100",
                            },
                        ]
                    },
                },
                {"uusd": "100"},
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    terra.tx.broadcast(stage_and_mint_tx)

    lp_tokens = terra.wasm.contract_query(
        lp_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - LP token balance after adding liquidity {lp_tokens}")

    print(f"[deploy] - bond lp tokens to staking contract")
    execute_contract(
        deployer,
        lp_token,
        CW20.send(staking_contract, "100", {"bond": {"asset_token": basket_token}}),
        seq(),
    )

    neb_balance = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - nebula token balance before depositing reward - {neb_balance}")

    print(f"[deploy] - deposit reward into staking contract")
    execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            staking_contract,
            "100",
            {"deposit_reward": {"rewards": [[basket_token, "100"]]}},
        ),
        seq(),
    )

    neb_balance = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - nebula token balance after depositing reward - {neb_balance}")

    print(f"[deploy] - withdraw reward from staking contract")
    execute_contract(
        deployer, staking_contract, {"withdraw": {"asset_token": basket_token}}, seq()
    )

    neb_balance = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - nebula token balance after withdrawing reward - {neb_balance}")


deploy()
