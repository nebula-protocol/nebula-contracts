import os
import asyncio
import requests
import json
import time

os.environ["MNEMONIC"] = mnemonic = 'idea salute sniff electric lecture table flag oblige pyramid light ocean heart web ramp save fiscal sting course uncle deputy way field vacant genius'

os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

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
        target_weights = [int(10000 * target_weight) for target_weight in target_weights]

        print(self.asset_tokens, target_weights)


        target = []
        for a, t in zip(self.asset_tokens, target_weights):
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

async def get_terra_ecosystem_info():
    ANC_ADDR = 'terra1mst8t7guwkku9rqhre4lxtkfkz3epr45wt8h0m'
    MIR_ADDR = 'terra159nvmamkrj0hw5e0e0lp4vzh6py0ev765jgl58'
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
    # luna_total_supply = coins_total_supply.get('uluna').amount
    luna_total_supply = 99453326200000
    asset_token_supply = [
        luna_total_supply/100000,
        anc_total_supply/ONE_MILLION,     # ANC
        mir_total_supply/ONE_MILLION      # MIR
    ]
    print(asset_token_supply)

    return assets, asset_token_supply

async def run_retarget_periodically(cluster_contract, interval):
    start_time = time.time()

    assets, asset_token_supply = get_terra_ecosystem_info()
    
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