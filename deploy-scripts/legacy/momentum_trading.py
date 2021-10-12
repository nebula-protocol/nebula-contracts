# Let It Roll bot source code


import requests
import json
from .graphql_querier import mirror_history_query, get_all_mirror_assets
import time
import pandas as pd
import asyncio


"""
Recomposes according to Total Value Locked (TVL) in the provided assets. 
WARN: Do not use with Mirrored Assets.
"""
class MomentumTradingRecomposer:
    def __init__(self, lookback_days=30, top=5):
        self.lookback_days = lookback_days
        self.top = top
        self.curr_asset_names = None
        self.curr_asset_addresses = None
        self.curr_asset_weights = None

    async def weighting(self):
        addresses = await get_all_mirror_assets()

        to = round(time.time() * 1000)

        MINUTES_PER_DAY = 1440
        minutes = self.lookback_days * 1440

        # Convert to millesconds
        from_time = to - minutes * 1000 * 60

        data = [await mirror_history_query(a, MINUTES_PER_DAY, from_time, to) for a in addresses]
        
        asset_names, max_timestamps, closes, _ = zip(*data)

        names_to_changes = {}
        for name, close in zip(asset_names, closes):
            names_to_changes[name] = 100 * (float(close[-1]) - float(close[0])) / float(close[0])

        # get best performers
        top_k = dict(sorted(names_to_changes.items(), key=lambda item: -item[1])[:self.top])

        best_assets = top_k.keys()

        denom = sum(top_k.values())

        target_weights = [val / denom for val in top_k.values()]

        name_to_addr = {name: addrs for name, addrs in zip(asset_names, addresses)}
        target_assets = [name_to_addr[b_a] for b_a in best_assets]

        return target_assets, target_weights, best_assets

        
    async def recompose(self):
        target_assets, target_weights, asset_names = await self.weighting()

        print (asset_names, target_weights)
        self.curr_asset_names = asset_names
        self.curr_asset_addresses = target_assets
        self.curr_asset_weights = target_weights

        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        return target_assets, target_weights

if __name__ == "__main__":
    rec_bot = MomentumTradingRecomposer()
    asyncio.get_event_loop().run_until_complete(rec_bot.recompose())
