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
import pprint

from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
terra = get_terra()

def mint(wBTC, wETH, basket):
    print("[deploy] - try to mint")
    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                wBTC,
                {"increase_allowance": {"spender": basket, "amount": "10000000"}},
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                wETH,
                {"increase_allowance": {"spender": basket, "amount": "10000000"}},
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                basket,
                Basket.mint(
                    [Asset.asset(wBTC, "10000000"), Asset.asset(wETH, "10000000")],
                    min_tokens="9990000",
                ),
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )
    result = terra.tx.broadcast(stage_and_mint_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)
    return result

def init_mint(wBTC, wETH, basket):
    print("[deploy] - initialize basket with tokens via mint")
    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                wBTC,
                {"increase_allowance": {"spender": basket, "amount": "100"}},
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                wETH,
                {"increase_allowance": {"spender": basket, "amount": "100"}},
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
    return result


def redeem(basket, basket_token, wBTC, wETH):
    print("[deploy] - basket:burn")

    redeem_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                basket_token,
                {"increase_allowance": {"spender": basket, "amount": "100"}},
            ),
            MsgExecuteContract(
                deployer.key.acc_address,
                basket,
                Basket.burn(
                    "100",
                    [
                        Asset.asset(wBTC, "1"),
                        Asset.asset(wETH, "2"),
                    ]
                )
            )
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )
    result = terra.tx.broadcast(redeem_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)
    return result


def add_liquidity(basket_token, pair_contract):
    print(f"[deploy] - adding liquidity to basket pair contract")
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

    result = terra.tx.broadcast(stage_and_mint_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)
    return result


def basket_operations(wBTC, wETH, basket_token, collector_contract, pair_contract, basket, nebula_token):

    result = init_mint(wBTC, wETH, basket)

    result = mint(wBTC, wETH, basket)

    basket_tokens = terra.wasm.contract_query(
        basket_token, {"balance": {"address": collector_contract}}
    )
    print(f"[deploy] - after minting collector has {basket_tokens}")

    debug_print = terra.wasm.contract_query(
        basket, {"basket_state": {"basket_contract_address": basket}}
    )
    print("Basket state after initialize --", debug_print)

    result = add_liquidity(basket_token, pair_contract)
    #
    # # COLLECTOR SWAPS BASKET INTO NEBULA
    # print(f"[deploy] telling collector to convert basket to uusd")
    # execute_contract(
    #     deployer, collector_contract, {"convert": {"asset_token": basket_token}}, seq()
    # )
    #
    # basket_tokens = terra.wasm.contract_query(
    #     basket_token, {"balance": {"address": collector_contract}}
    # )
    #
    # print(f"[deploy] - collector basket balance {basket_tokens}")
    # print(f"[deploy] telling collector to convert uusd to nebula")
    # execute_contract(
    #     deployer, collector_contract, {"convert": {"asset_token": nebula_token}}, seq()
    # )
    #
    # nebula_tokens = terra.wasm.contract_query(
    #     nebula_token, {"balance": {"address": collector_contract}}
    # )
    #
    # print(f"[deploy] - collector nebula balance {nebula_tokens}")
    #
    # basket_tokens = terra.wasm.contract_query(
    #     basket_token, {"balance": {"address": deployer.key.acc_address}}
    # )
    # print("Before redeem", basket_tokens)
    # redeem(basket, basket_token, wBTC, wETH)
    #
    # basket_tokens = terra.wasm.contract_query(
    #     basket_token, {"balance": {"address": deployer.key.acc_address}}
    # )
    #
    # print("After redeem", basket_tokens)