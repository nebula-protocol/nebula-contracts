import os
import asyncio
import requests
import json
import time

os.environ["MNEMONIC"] = mnemonic = 'idea salute sniff electric lecture table flag oblige pyramid light ocean heart web ramp save fiscal sting course uncle deputy way field vacant genius'

os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

from .pricing import get_query_info, get_prices

ONE_MILLION = 1000000.0
SECONDS_PER_DAY = 24 * 60 * 60

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
        target_weights = [asset_to_fdm[asset]/denom for asset in asset_to_fdm]
        print(self.asset_tokens, target_weights)
        _, _, query_info = await get_query_info(self.asset_tokens)
        prices = await get_prices(query_info)
        target = []
        for a, t, p in zip(self.asset_tokens, target_weights, prices):
            native = (a == 'uluna')
            print(t)
            tw = str(int(10000000 * t / float(p)))
            target.append(Asset.asset(a, tw, native=native))

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

async def get_terra_ecosystem_info():
    ANC_ADDR = 'terra1jffn63c4tfzg66qdcaznhqaphmgvprp7muauq6'
    MIR_ADDR = 'terra1vx0esu27cfkswurt646x3mhfh4wvlwpf4g5t6l'
    assets = [
        "uluna", #LUNA
        ANC_ADDR, #ANC
        MIR_ADDR #MIR
    ]
    anc_token = Contract(ANC_ADDR) 

    # May not be accurate on Tequila
    anc_token_info = await anc_token.query.token_info()
    anc_total_supply = float(anc_token_info['total_supply'])

    mir_token = Contract(MIR_ADDR)
    mir_token_info = await mir_token.query.token_info()
    mir_total_supply = float(mir_token_info['total_supply'])

    # coins_total_supply = await terra.supply.total()
    # coins_total_supply = await terra.bank.total()
    # luna_total_supply = coins_total_supply.get('uluna').amount
    luna_total_supply = 105996364234582
    asset_token_supply = [
        luna_total_supply/100000,
        anc_total_supply/ONE_MILLION,     # ANC
        mir_total_supply/ONE_MILLION      # MIR
    ]
    print(asset_token_supply)

    return assets, asset_token_supply

async def run_retarget_periodically(cluster_contract, interval):
    start_time = time.time()

    assets, asset_token_supply = await get_terra_ecosystem_info()
    
    retarget_bot = TerraFullDilutedMcapRecomposer(cluster_contract, assets, asset_token_supply)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            retarget_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1pk8069vrxm0lzqdqy6zq46np8e4jy3r7j0a5k9")
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_retarget_periodically(cluster_contract, interval))