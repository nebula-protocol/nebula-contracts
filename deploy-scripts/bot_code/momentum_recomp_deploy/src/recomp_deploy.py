import os
import asyncio
import requests
import json
import time

from graphql_querier import mirror_history_query_test, get_all_mirror_assets_test
import time
import pandas as pd

os.environ["MNEMONIC"] = mnemonic = 'idea salute sniff electric lecture table flag oblige pyramid light ocean heart web ramp save fiscal sting course uncle deputy way field vacant genius'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

SECONDS_PER_DAY = 24 * 60 * 60

"""
Recomposes according to Momentum and tracks the top 5 best-performing mAssets.
"""
class MomentumTradingRecomposer:
    def __init__(self, cluster_contract, lookback_days=30, top=5):
        self.cluster_contract = cluster_contract
        self.lookback_days = lookback_days
        self.top = top
        self.curr_asset_names = None
        self.curr_asset_addresses = None
        self.curr_asset_weights = None

    async def weighting(self):
        addresses = await get_all_mirror_assets_test()

        to = round(time.time() * 1000)

        MINUTES_PER_DAY = 1440
        minutes = self.lookback_days * 1440

        # Convert to millesconds
        from_time = to - minutes * 1000 * 60

        data = [await mirror_history_query_test(a, MINUTES_PER_DAY, from_time, to) for a in addresses]
        asset_names, max_timestamps, closes, _ = zip(*data)

        names_to_changes = {}
        for name, close in zip(asset_names, closes):
            if name and close:
                names_to_changes[name] = 100 * (float(close[-1]) - float(close[0])) / float(close[0])
            else:
                names_to_changes[name] = float('-inf')

        # get best performers
        top_k = dict(sorted(names_to_changes.items(), key=lambda item: -item[1])[:self.top])

        best_assets = top_k.keys()

        denom = sum(top_k.values())

        target_weights = [val / denom for val in top_k.values()]

        name_to_addr = {name: addrs for name, addrs in zip(asset_names, addresses)}
        target_assets = [name_to_addr[b_a] for b_a in best_assets]

        return target_assets, target_weights, best_assets
    
    async def recompose(self):

        target_assets, target_weights, best_assets = await self.weighting()
        print('Best assets', best_assets)
        print('Target weights', target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        await self.cluster_contract.reset_target(
            assets=[Asset.asset_info(a) for a in target_assets],
            target=target_weights
        )

        target = await self.cluster_contract.query.target()
        cluster = Contract("terra1wa7frpp078hnqnlvevmqjyswvnswp4psmkjred")
        cluster_state = await self.cluster_contract.query.cluster_state(
            cluster_contract_address=cluster
        )

        print("Updated Target: " , target)
        print("Updated Cluster State: ", cluster_state)
        return target_assets, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    start_time = time.time()
    
    recomposition_bot = MomentumTradingRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1wa7frpp078hnqnlvevmqjyswvnswp4psmkjred")
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))