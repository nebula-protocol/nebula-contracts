from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

import asyncio
import os


USE_TEQUILA = bool(os.environ.get("USE_TEQUILA"))
USE_MNEMONIC = bool(os.environ.get("MNEMONIC"))

CACHE_INITIALIZATION = True
OVERWRITE_CACHE_ALLOWED = set()


lt = AsyncLocalTerra(gas_prices={"uusd": "0.15"})

if USE_MNEMONIC:
    key = MnemonicKey(mnemonic=os.environ.get("MNEMONIC"))
    print('using mnemonic')
else:
    key = lt.wallets["test1"].key

if USE_TEQUILA:
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
    deployer = terra.wallet(key)
else:
    terra = lt
    deployer = lt.wallets["test1"]

sequence = asyncio.get_event_loop().run_until_complete(deployer.sequence())

print(deployer.key.acc_address)
async def sign_and_broadcast(*msgs):

    global sequence
    try:
        tx = await deployer.create_and_sign_tx(
            msgs=msgs, gas_prices={"uusd": "0.15"}, gas_adjustment=1.5, sequence=sequence
        )
        result = await terra.tx.broadcast(tx)
        sequence += 1
        if result.is_tx_error():
            raise Exception(result.raw_log)
        return result
    except:
        sequence = await deployer.sequence()
        raise
