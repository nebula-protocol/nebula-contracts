import os
import asyncio
import requests
import json
import time

from graphql_querier import mirror_history_query, get_all_mirror_assets
import time
import pandas as pd

os.environ["MNEMONIC"] = mnemonic = 'raise sight lemon exact duty master buyer drink runway trap tourist use camp habit crunch horror jeans rice dance castle lift couple raise vibrant'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

THRESHOLD = 0.5
# Percentage to deweight non-cross assets
X = 0.2
API_KEY = os.environ.get("AV_API", None)

"""
Recomposes according to Fully Diluted Market Cap in the terra ecosystem assets. 
"""
class BullishCrossRecomposer:
    def __init__(self, cluster_contract, minperiod=5, maxperiod=50, lookahead = 200, bar_length = 30):
        """ 
        Defaults to 30min tick and 200 bars of data (around 4 days).
        Asset names can also be addresses
        """
        self.minperiod=minperiod 
        self.maxperiod=maxperiod

        self.cluster_contract = cluster_contract

        self.lookahead = lookahead
        self.bar_length = bar_length

    def self_opt_ma(self, data):
        """
        The sample implementation (code at the end of the article) will calculate all moving 
        averages within a given parameter range (eg. 5 bars to 200 bars), calculate the winning 
        percentage (rising bars) on the next bar, and then pick the best performing period length.
        """

        close = data[-self.lookahead:].reset_index(drop=True)
        # yesterday
        c1 = data[-self.lookahead- 1 : -1].reset_index(drop=True)
        # 3 days ago
        c3 = data[-self.lookahead - 3 : -3].reset_index(drop=True)

        counter_list = []
        win_list = []
        win_percentage_list = []

        # loop over all MA candidates
        for i in range(self.minperiod, self.maxperiod+1):
            av_c1 = c1.rolling(i).mean().reset_index(drop=True)
            av_c3 = c3.rolling(i).mean().reset_index(drop=True)
            # Total count: times av_c1 beats av_c3
            counter = (av_c1 > av_c3).sum()
            # Wins: When current close > yesterday's with av_c1 > av_c3 signal
            wins = ((av_c1 > av_c3) & (close > c1)).sum()
            counter_list.append(counter)
            win_list.append(wins)
            if counter != 0:
                win_percentage_list.append(wins / counter)
            else:
                win_percentage_list.append(0)

        smoothed_win_percentage_list = []
        # smoothing
        for j in range(2, len(win_percentage_list) - 2 - 1):
            smoothed_win_percentage = sum(win_percentage_list[j-2 : j + 2 + 1])/5
            smoothed_win_percentage_list.append(smoothed_win_percentage)

        # choose best MA candidate after smoothing
        best_period = self.minperiod + 2 + max(enumerate(smoothed_win_percentage_list), key=lambda x: x[1])[0]
        best_pwp = max(smoothed_win_percentage_list)
        av = close.rolling(best_period).mean().reset_index(drop=True)
        price_gt_ma = close.iat[-1] > av.iat[-1]
        return best_pwp, price_gt_ma

    def get_mcaps(self, asset_names):
        """
        Get actual stock market caps corresponding to mAsset
        """

        if API_KEY is None:
            raise NameError

        mcs = []

        for name in asset_names:
            try:
                stock = name[1:]
                url = 'https://www.alphavantage.co/query?function=OVERVIEW&symbol={}&apikey={}'.format(stock, API_KEY)
                r = requests.get(url)
                data = r.json()

                mc = float(data['MarketCapitalization'])
        
                if mc < 20000000000:
                    mc = 0
                    
                mcs.append(mc)
            except:
                mcs.append(0)
        
        return mcs

    async def cross_weighting(self):

        self.asset_addresses = await get_all_mirror_assets()

        has_cross, all_cross = False, True
        best_pwps = {}
        non_cross_assets = []
        asset_addresses = self.asset_addresses

        to = round(time.time() * 1000)

        # Need at least (self.lookahead + 3) pieces of historical data
        from_time = to - (self.lookahead + 4) * self.bar_length * 1000 * 60

        data = [await mirror_history_query(a, self.bar_length, from_time, to) for a in self.asset_addresses]

        data = [d for d in data if d[0] is not None]
        # Might need asset names from CMC
        asset_names, max_timestamps, closes, _ = zip(*data)

        # Calculate MC of actual asset names
        mcs = self.get_mcaps(asset_names)

        asset_data = {name: int(mc) for name, mc in zip(asset_names, mcs)}
        self.closes = {name: pd.Series(close).astype('float') for name, close in zip(asset_names, closes)}
        names_to_contracts = {name: addrs for name, addrs in zip(asset_names, self.asset_addresses)}

        # Calculate best_pwps and assets with crosses
        for asset, asset_closes in self.closes.items():
            best_pwp, price_gt_ma = self.self_opt_ma(asset_closes)
            if price_gt_ma and best_pwp > THRESHOLD:
                has_cross = True
                best_pwps[asset] = best_pwp
            else:
                all_cross = False
                non_cross_assets.append(asset)
        
        asset_mcaps = list(asset_data.values())
        print(asset_names, asset_mcaps)
        # All assets have crosses
        if all_cross:
            diffs = [best_pwps[asset] - THRESHOLD for asset in asset_names]
            denom = sum([asset_mcaps[i] * diffs[i] for i in range(len(asset_names))])
            target = {asset_names[i]: (asset_mcaps[i] * diffs[i])/denom for i in range(len(asset_names))}
        else:
            denom = sum(asset_mcaps)
            target = {asset_names[i]: asset_mcaps[i]/denom for i in range(len(asset_names))}
            print("Original allocation: {}".format(target))

            # Some asset has a cross, then divide up X share of non-cross assets to cross assets
            if has_cross:
                non_cross_pool = 0
                for asset_name in non_cross_assets:
                    share = X * target[asset_name]
                    non_cross_pool += share
                    target[asset_name] -= share
                    print("{} does not have a cross. Pool takes {}".format(asset_name, share))
                print("Total Non-Cross Pool: {}".format(non_cross_pool))
                cross_assets = list(best_pwps.keys())
                cross_diffs = [best_pwps[asset] - THRESHOLD for asset in cross_assets]
                cross_asset_mcaps = [asset_data[asset] for asset in cross_assets]
                denom = sum([cross_asset_mcaps[i] * cross_diffs[i] for i in range(len(cross_assets))])
                for i in range(len(cross_assets)):
                    cross_asset = cross_assets[i]
                    new_weight = (cross_asset_mcaps[i] * cross_diffs[i])/denom
                    target[cross_asset] += new_weight * non_cross_pool
                    print("{} target weight updated to {}".format(cross_asset, new_weight))
        assets, target_weights = zip(*target.items())
        addresses = [names_to_contracts[a] for a in assets]

        return list(addresses), list(target_weights), list(assets)
    
    async def recompose(self):
        addresses, target_weights, names = await self.cross_weighting()
        print('Best assets', names)
        print('Target weights', target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        await self.cluster_contract.reset_target(
            assets=[Asset.asset_info(a) for a in addresses],
            target=target_weights
        )

        target = await self.cluster_contract.query.target()
        cluster = Contract("terra1xtn56y9pd7vrm7fwswtw7k4zctnm7ld9xv3p3l")
        cluster_state = await self.cluster_contract.query.cluster_state(
            cluster_contract_address=cluster
        )

        print("Updated Target: " , target)
        print("Updated Cluster State: ", cluster_state)
        return addresses, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    start_time = time.time()
    
    recomposition_bot = BullishCrossRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1xtn56y9pd7vrm7fwswtw7k4zctnm7ld9xv3p3l")
    interval = 24 * 60 * 60
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))