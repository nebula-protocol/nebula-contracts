import os
import asyncio
import requests
import json

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract

mnemonic = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

key = MnemonicKey(mnemonic=mnemonic)
print('using mnemonic')

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


"""
Recomposes according to Total Value Locked (TVL) in the provided assets. 
WARN: Do not use with Mirrored Assets.
"""
class TerraFullDilutedMcapRecomposer:
    def __init__(self, cluster_contract, asset_names, asset_tokens):
        self.cluster_contract = cluster_contract
        self.asset_names = asset_names
        self.asset_tokens = asset_tokens
        self.asset_ids = [
            "terra-luna",
            "anchor-protocol",
            "mirror-protocol"
        ]
        self.api = "https://api.coingecko.com/api/v3/simple/price"
        self.currency = "usd"

    async def weighting(self):
        get_params = {
            "ids": ','.join(self.asset_ids),
            "vs_currencies": self.currency
        }
        r = requests.get(self.api, params=get_params)
        r.raise_for_status()
        prices = json.loads(r.text)
        asset_to_fdm = {}
        for i in range(len(self.asset_ids)):
            token_contract = self.asset_tokens[i]
            token_info = await token_contract.query.token_info()
            asset_id = self.asset_ids[i]
            price = float(prices[asset_id][self.currency])
            total_supply = float(token_info['total_supply'])
            fully_diluted_mcap = price * total_supply
            asset_to_fdm[asset_id] = fully_diluted_mcap
            print("{} has TVL of {}M".format(asset_id, fully_diluted_mcap/1000000.0))
        denom = sum(asset_to_fdm.values())
        print("Total FDM of all assets: {}M".format(denom/1000000.0))
        target = [asset_to_fdm[asset]/denom for asset in asset_to_fdm]
        return target
        
    
    async def recompose(self):
        target_weights = await self.weighting()
        print(self.asset_names, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]

        await self.cluster_contract.reset_target(
            assets=[Asset.cw20_asset_info(a) for a in self.asset_names],
            target=target_weights
        )

        return self.asset_names, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    start_time = time.time()
    assets = ["LUNA", "MIR", "ANC"]
    asset_tokens = [] #Add respective token addresses
    recomposition_bot = TerraFullDilutedMcapRecomposer(cluster_contract, assets, asset_tokens)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("TODO")
    interval = 24 * 60 * 60
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))