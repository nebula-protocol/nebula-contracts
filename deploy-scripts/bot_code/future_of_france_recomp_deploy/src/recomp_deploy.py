import os
import asyncio
import requests
import json

os.environ["MNEMONIC"] = mnemonic = 'lottery horn blast wealth cruise border opinion upgrade common gauge grocery evil canal lizard sad mad submit degree brave margin age lunar squirrel diet'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

ONE_MILLION = 1000000.0
SECONDS_PER_DAY = 24 * 60 * 60

"""
Recomposes according to Total Value Locked (TVL) in the provided assets. 
WARN: Do not use with Mirrored Assets.
"""
class FutureOfFranceRecomposer:
    def __init__(self, cluster_contract):
        self.cluster_contract = cluster_contract
        self.asset_names = ["AAVE", "COMP", "MKR", "CREAM", "ANC"]
        self.api = "https://api.llama.fi"
        self.protocol_endpoint = "protocols"
        self.tvl_endpoint = "tvl"
        self.slugs = self.get_slugs(self.asset_names)

    def get_slugs(self, asset_names):
        protocol_url = "{}/{}".format(self.api, self.protocol_endpoint)
        r = requests.get(protocol_url)
        r.raise_for_status()
        protocols = json.loads(r.text)
        symbol_to_slug = {}
        for protocol in protocols:
            symbol = protocol["symbol"]
            slug = protocol["slug"]
            if symbol not in symbol_to_slug:
                symbol_to_slug[symbol] = slug
        
        slugs = []
        for asset_name in asset_names:
            if asset_name not in symbol_to_slug:
                print("{} not found in {}".format(asset_name, self.api))
            slug = symbol_to_slug[asset_name]
            slugs.append(slug)
        return slugs

    async def weighting(self):
        tvl_url = self.api + self.tvl_endpoint
        tvls = {}
        for slug in self.slugs:
            tvl_url = "{}/{}/{}".format(self.api, self.tvl_endpoint, slug)
            r = requests.get(tvl_url)
            r.raise_for_status()
            tvl = float(r.text)
            tvls[slug] = tvl
            print("{} has TVL of {}M".format(slug, tvl/ONE_MILLION))
        denom = sum(tvls.values())
        print("Total TVL of all assets: {}M".format(denom/ONE_MILLION))
        target = [tvls[slug]/denom for slug in self.slugs]
        asset_addresses = self.asset_names
        #TODO: Add mapping from asset names to contract addresses
        return asset_addresses, target
        
    
    async def recompose(self):
        assets, target_weights = await self.weighting()
        print(self.asset_names, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        # await self.cluster_contract.reset_target(
        #     assets=[Asset.asset_info(a) for a in self.asset_tokens],
        #     target=target_weights
        # )
        # target = await self.cluster_contract.query.target()
        # print("Updated Target: " , target)

        return self.asset_names, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    recomposition_bot = FutureOfFranceRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("") #TODO: Update
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))