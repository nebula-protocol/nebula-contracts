import os
import asyncio
import requests
import json

os.environ["MNEMONIC"] = mnemonic = 'trash comic lawn fatal jewel alien twin drip immense general rose ahead coffee rack liquid bottom because unveil clean butter leave wear slam surprise'
os.environ["USE_TEQUILA"] = "1"

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
        self.assets_to_address = {
            'AAVE': 'terra1exw6sae4wyq8rt56hxdggzmgmqsuukr26u4aj8',
            'ANC': 'terra1mst8t7guwkku9rqhre4lxtkfkz3epr45wt8h0m',
            'COMP': 'terra10af2zy62wanc6cs3n66cplmpepvf6qnetuydz2',
            'CREAM': 'terra1a7g946jyjhn8h7gscda7sd68kn9k4whkxq0ddn',
            'MKR': 'terra1lflvesvarcfu53gd9cgkv3juyrz79cnk7yw6am'
        }
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
        target_weights = [tvls[slug]/denom for slug in self.slugs]
        asset_tokens = [self.assets_to_address[an] for an in self.asset_names]
        print(self.asset_names, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]

        target = []
        for a, t in zip(asset_tokens, target_weights):
            native = (a == 'uluna')
            target.append(Asset.asset(a, str(t), native=native))
        return target
        
    
    async def recompose(self):
        target = await self.weighting()
        
        print(target)

        await self.cluster_contract.update_target(
            target=target
        )

        target = await self.cluster_contract.query.target()
        cluster_state = await self.cluster_contract.query.cluster_state(
            cluster_contract_address=self.cluster_contract
        )

        print("Updated Target: " , target)
        print("Updated Cluster State: ", cluster_state)
        return target

async def run_retarget_periodically(cluster_contract, interval):
    retarget_bot = FutureOfFranceRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            retarget_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1fx9m968gn53cu92qf8ye9s4v0nznllkg9w79vp")
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_retarget_periodically(cluster_contract, interval))