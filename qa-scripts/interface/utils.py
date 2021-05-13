from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

from .api import Oracle, Basket, CW20, Asset
from functools import _make_key
import shelve

import asyncio

CACHE_INITIALIZATION = True
OVERWRITE_CACHE_ALLOWED = set()


USE_LOCALTERRA = True

lt = AsyncLocalTerra(gas_prices={"uusd": "0.15"})

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

    terra = AsyncLCDClient(
        "https://tequila-fcd.terra.dev", "tequila-0004", gas_prices=gas_prices
    )

    deployer = terra.wallet(lt.wallets["test1"].key)

sequence = asyncio.get_event_loop().run_until_complete(deployer.sequence())


def seq():
    """Increments global sequence."""
    global sequence
    sequence += 1
    return sequence - 1


shelf = shelve.open("cache.dat")

# General contract helper functions


def async_cache_on_disk(fxn):
    async def _ret(*args, **kwargs):
        key = repr(args) + "|" + repr(kwargs)
        key = fxn.__name__ + "|" + str(key)
        if key not in shelf or fxn.__name__ in OVERWRITE_CACHE_ALLOWED:
            shelf[key] = await fxn(*args, **kwargs)
            shelf.sync()
        return shelf[key]

    return _ret if CACHE_INITIALIZATION else fxn


@async_cache_on_disk
async def store_contract(contract_name):
    import os

    parent_dir = os.path.dirname(os.path.dirname(os.path.dirname(__file__)))
    contract_bytes = read_file_as_b64(f"{parent_dir}/artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)
    store_code_tx = await deployer.create_and_sign_tx(
        msgs=[store_code], fee=StdFee(5000000, "2000000uusd"), sequence=seq()
    )
    result = await terra.tx.broadcast(store_code_tx)
    if result.is_tx_error():
        print(result.raw_log)

    code_id = get_code_id(result)
    print(f"Code id for {contract_name} is {code_id}")
    return code_id


async def instantiate_contract(code_id, init_msg):
    instantiate = MsgInstantiateContract(deployer.key.acc_address, code_id, init_msg)
    instantiate_tx = await deployer.create_and_sign_tx(
        msgs=[instantiate], sequence=seq(), fee_denoms=["uusd"]
    )
    result = await terra.tx.broadcast(instantiate_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)
    return get_contract_address(result)


async def execute_contract(contract_address, execute_msg, fee=None):
    execute = MsgExecuteContract(
        deployer.key.acc_address, contract_address, execute_msg
    )
    execute_tx = await deployer.create_and_sign_tx(
        msgs=[execute], sequence=seq(), fee_denoms=["uusd"], fee=fee
    )
    result = await terra.tx.broadcast(execute_tx)
    if result.is_tx_error():
        raise Exception(result.raw_log)
    return result


@async_cache_on_disk
async def instantiate_token_contract(token_code_id, name, symbol, initial_amount):
    return await instantiate_contract(
        token_code_id,
        {
            "name": name,
            "symbol": symbol,
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": initial_amount}
            ],
            "mint": None,
        },
    )


@async_cache_on_disk
async def instantiate_oracle(oracle_code_id, assets, initial_prices):
    oracle = await instantiate_contract(oracle_code_id, {})

    await execute_contract(
        oracle,
        Oracle.set_prices(list(zip(assets, initial_prices))),
    )
    return oracle


async def create_basket(
    basket_tokens,
    asset_tokens,
    asset_prices,
    target_weights,
    penalty_params,
):

    basket_tokens = str(basket_tokens)
    asset_tokens = tuple(str(i) for i in asset_tokens)
    asset_prices = tuple(str(i) for i in asset_prices)
    target_weights = tuple(int(i) for i in target_weights)
    penalty_params = {k: str(v) for k, v in penalty_params.items()}

    print("Creating basket...")
    print("Storing contracts...")
    basket_code_id = await store_contract("basket_contract")
    token_code_id = await store_contract("terraswap_token")
    oracle_code_id = await store_contract("basket_dummy_oracle")
    penalty_code_id = await store_contract("basket_penalty")
    factory_code_id = await store_contract("basket_factory")
    terraswap_factory_code_id = await store_contract("terraswap_factory")
    pair_code_id = await store_contract("terraswap_pair")
    staking_code_id = await store_contract("basket_staking")

    terraswap_factory_contract = await instantiate_contract(
        terraswap_factory_code_id,
        {"pair_code_id": int(pair_code_id), "token_code_id": int(token_code_id)},
    )

    print("Creating factory...")
    factory_contract = await instantiate_contract(
        factory_code_id,
        {
            "token_code_id": int(token_code_id),
            "cluster_code_id": int(basket_code_id),
            "base_denom": "uusd",
            "protocol_fee_rate": "0.001",
            "distribution_schedule": [[0, 100000, "1000000"]],
        },
    )

    print("Creating nebula token...")
    nebula_token = await instantiate_contract(
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
                {
                    "address": factory_contract,
                    "amount": "10000000000",
                },
            ],
            # maybe ?
            "minter": {"minter": factory_contract, "cap": None},
        },
    )

    print(f"Create staking contract")
    staking_contract = await instantiate_contract(
        staking_code_id,
        {
            "owner": factory_contract,
            "nebula_token": nebula_token,
            "terraswap_factory": terraswap_factory_contract,
            "base_denom": "uusd",
            "premium_min_update_interval": 5,
        },
    )

    await execute_contract(
        factory_contract,
        {
            "post_initialize": {
                "owner": deployer.key.acc_address,
                "nebula_token": nebula_token,
                "oracle_contract": nebula_token,  # ??? provide arbitrary contract for now
                "terraswap_factory": terraswap_factory_contract,
                "staking_contract": staking_contract,
                "commission_collector": nebula_token,
            }
        },
    )

    print("Creating asset tokens...")
    assets = [
        (
            await instantiate_token_contract(
                token_code_id,
                f"Asset {i}",
                f"AA{chr(i + 97)}",
                "1" + "0" * 15,
            )
        )
        for i in range(len(asset_tokens))
    ]

    assets = tuple(assets)
    print("Creating oracle...")
    oracle = await instantiate_oracle(oracle_code_id, assets, asset_prices)

    print(f"Creating penalty contract")
    penalty_contract = await instantiate_contract(
        penalty_code_id,
        {"penalty_params": penalty_params, "owner": factory_contract},
    )

    resp = await execute_contract(
        factory_contract,
        {
            "create_cluster": {
                "name": "BASKET",
                "symbol": "BSK",
                "params": {
                    "name": "BASKET",
                    "symbol": "BSK",
                    "penalty": penalty_contract,
                    "target": target_weights,
                    "assets": [Asset.cw20_asset_info(i) for i in assets],
                    "oracle": oracle,
                },
            }
        }
    )
    if resp.is_tx_error():
        raise Exception(resp.raw_log)

    logs = resp.logs[0].events_by_type

    instantiation_logs = logs["instantiate_contract"]
    addresses = instantiation_logs["contract_address"]

    basket = addresses[3]
    basket_token = addresses[2]

    print("Basket details", basket, basket_token, assets)
    return basket, basket_token, assets
