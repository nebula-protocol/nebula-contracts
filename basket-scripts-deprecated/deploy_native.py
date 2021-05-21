"""Sample deploy script.

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

from terra_sdk.client.lcd import LCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.bank.msgs import MsgSend
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

from basket import Oracle, Basket, CW20, Asset

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

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


    # instantiate oracle
    print(f"[deploy] - instantiate oracle")
    oracle = instantiate_contract(oracle_code_id, {}, seq())

    # instantiate basket
    print(f"[deploy] - instantiate basket")
    basket = instantiate_contract(
        basket_code_id,
        {
            "name": "Basket",
            "owner": deployer.key.acc_address,
            "assets": [Asset.native_asset_info('uluna')],
            "oracle": oracle,
            "penalty_params": {
                "a_pos": "1",
                "s_pos": "1",
                "a_neg": "0.005",
                "s_neg": "0.5",
            },
            "target": [100],
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

    # set oracle prices
    print(f"[deploy] - set oracle prices")
    execute_contract(
        deployer,
        oracle,
        Oracle.set_prices(
            [
                ['uluna', "15.00"],
            ]
        ),
        seq(),
    )

    # sets initial balance of basket contract
    amount_uluna = "1000000"

    print(
        f"[deploy] - give initial balances uluna {amount_uluna}"
    )

    # TODO 
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgSend(
                deployer.key.acc_address, basket, {"uluna": amount_uluna},
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)
    print(result.logs[0].events_by_type)


    ### EXAMPLE: how to query basket state
    print("FIRST")
    print(
        terra.wasm.contract_query(
            basket, {"basket_state": {"basket_contract_address": basket}}
        )
    )

    print("BALANCE")
    print(
        terra.wasm.contract_query(
            basket_token, {"balance": {"address": deployer.key.acc_address}}
        )
    )

    ### EXAMPLE: how to stage and mint

    print("[deploy] - basket:stage_asset")
    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                basket,
                Basket.stage_native_asset("uluna", "100000"),
                {"uluna": 100000},
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )
    result = terra.tx.broadcast(stage_and_mint_tx)
    print(f"stage TXHASH: {result.txhash}")
    print(result.logs[0].events_by_type)

    ### EXAMPLE: how to query basket state
    print(
        terra.wasm.contract_query(
            basket, {"basket_state": {"basket_contract_address": basket}}
        )
    )

    ### EXAMPLE: how to stage and mint
    print("[deploy] - basket:mint")
    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                basket,
                Basket.mint([Asset.asset("uluna", "100000", native=True)]),
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )
    result = terra.tx.broadcast(stage_and_mint_tx)
    print(f"mint TXHASH: {result.txhash}")
    print(result.logs[0].events_by_type)

    print("BALANCE after")
    print(
        terra.wasm.contract_query(
            basket_token, {"balance": {"address": deployer.key.acc_address}}
        )
    )


    ### EXAMPLE: how to query basket state
    print(
        terra.wasm.contract_query(
            basket, {"basket_state": {"basket_contract_address": basket}}
        )
    )

deploy()