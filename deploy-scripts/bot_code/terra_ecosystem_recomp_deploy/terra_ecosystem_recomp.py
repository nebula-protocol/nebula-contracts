import os
import asyncio
import requests
import json
import time

os.environ["MNEMONIC"] = mnemonic = 'lottery horn blast wealth cruise border opinion upgrade common gauge grocery evil canal lizard sad mad submit degree brave margin age lunar squirrel diet'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

ONE_MILLION = 1000000.0

"""
Recomposes according to Fully Diluted Market Cap in the terra ecosystem assets. 
"""
class TerraFullDilutedMcapRecomposer:
    def __init__(self, cluster_contract, asset_tokens, asset_token_supply):
        self.cluster_contract = cluster_contract
        self.asset_tokens = asset_tokens
        self.asset_token_supply = asset_token_supply
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
            total_supply = self.asset_token_supply[i]
            asset_id = self.asset_ids[i]
            price = float(prices[asset_id][self.currency])
            fully_diluted_mcap = price * total_supply
            asset_to_fdm[asset_id] = fully_diluted_mcap
            print("{} has FDM of {}M".format(asset_id, fully_diluted_mcap/ONE_MILLION))
        denom = sum(asset_to_fdm.values())
        print("Total FDM of all assets: {}M".format(denom/ONE_MILLION))
        target = [asset_to_fdm[asset]/denom for asset in asset_to_fdm]
        return target
        
    
    async def recompose(self):
        target_weights = await self.weighting()
        print(self.asset_tokens, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]

        await self.cluster_contract.reset_target(
            assets=[Asset.asset_info(a) for a in self.asset_tokens],
            target=target_weights
        )

        target = await self.cluster_contract.query.target()
        cluster = Contract("terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5")
        cluster_state = await self.cluster_contract.query.cluster_state(
            cluster_contract_address=cluster
        )

        print("Updated Target: " , target)
        print("Updated Cluster State: ", cluster_state)
        return self.asset_tokens, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    start_time = time.time()
    assets = [
        "uluna", #LUNA
        "terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc", #ANC
        "terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u" #MIR
    ]
    anc_token = Contract("terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc") 
    anc_token_info = await anc_token.query.token_info()
    anc_total_supply = float(anc_token_info['total_supply'])
    mir_token = Contract("terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u")
    mir_token_info = await mir_token.query.token_info()
    mir_total_supply = float(mir_token_info['total_supply'])
    coins_total_supply = await terra.supply.total()
    luna_total_supply = coins_total_supply.get('uluna').amount
    asset_token_supply = [
        luna_total_supply/100000,
        anc_total_supply/ONE_MILLION,     # ANC
        mir_total_supply/ONE_MILLION      # MIR
    ]
    print(asset_token_supply)
    recomposition_bot = TerraFullDilutedMcapRecomposer(cluster_contract, assets, asset_token_supply)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5")
    interval = 24 * 60 * 60
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))