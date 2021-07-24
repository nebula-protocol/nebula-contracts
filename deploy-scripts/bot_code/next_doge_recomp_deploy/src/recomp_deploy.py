import os
import asyncio
import requests
import json
from datetime import timedelta, datetime

os.environ["MNEMONIC"] = mnemonic = 'parent hospital arrest brush exact giraffe glimpse exist grain curtain always depend session wash twin insane rural brain ahead destroy sudden claim story funny'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra

ONE_MILLION = 1000000.0
SECONDS_PER_DAY = 24 * 60 * 60

# The Next Doge Cluster Methodology
"""
For underlying tokens in a cluster, we define them as “activated“ and “deactivated” according to a binary activation function. 
Consider an activation function such that when the H = 4-hour price percentage for a token exceeds T = 30%, we mark it “activated”. 
This activation lasts for a period of D = 3 days and after that the token reverts to a “deactivated” state. Let’s say an “activated” 
token can be allocated a maximum of X = 20% of current UST reserves. Now, if one more “activated“ tokens exist, weights of the 
cluster are first calculated proportional to the market cap of each “activated” token, and then a function f(w) = max(w, X) is 
applied to each weight in the cluster. If there's any remaining weight percentage, it's allocated to UST. 
However, if no “activated” tokens exist, we assign a 100% weight to UST and wait for an activation event occurrence.
"""
class NextDogeRecomposer:
    def __init__(self, cluster_contract):
        self.cluster_contract = cluster_contract

        #["UST, ""DOGE", "CUMMIES", "MEME", "ERC20"]
        self.asset_ids = ["terrausd", "dogecoin", "cumrocket", "degenerator", "erc20"]
        self.asset_infos = [
            'uusd', 
            'terra1wpa2978x6n9c6xdvfzk4uhkzvphmq5fhdnvrym',
            'terra1kf9qa5f3uu7nq3flg2dva8c9d9lh8h5cyuextt',
            'terra1u08z2c9r3s3avrn9l0r3m30xhcmssunvv5d0rx',
            'terra1p0rp8he7jfnevha3k5anhd0als7azjmfhxrvjv',
        ]

        self.H = 8
        self.T = 0.3
        self.X = 0.1
        self.P = 24

        self.default_weights = [1.0] + [0 for i in range(len(self.asset_ids) - 1)]
        self.activated_assets = {}
        self.deactivated_assets = set()
        self.vs_currency = "usd"

    def activate_asset(self, asset, cur_timestamp):
        if asset in self.deactivated_assets:
            self.deactivated_assets.remove(asset)
        timedelay = timedelta(hours=self.P)
        self.activated_assets[asset] = datetime.utcfromtimestamp(cur_timestamp) + timedelay

    def deactivate_asset(self, asset):
        if asset in self.activated_assets:
            self.activated_assets.pop(asset)
        self.deactivated_assets.add(asset)

    def try_deactivate_expired(self, cur_timestamp):
        asset_list = list(self.activated_assets.keys())
        for asset in asset_list:
            if self.activated_assets[asset] <= datetime.utcfromtimestamp(cur_timestamp):
                self.deactivate_asset(asset)
                
    # Example: get_price_change("terra-luna", "usd", 1557270594, 1557288000)
    def get_price_change(self, asset_id, time_from, time_to):
        api_url = "https://api.coingecko.com/api/v3/coins/{id}/market_chart/range".format(
            id=asset_id
        )
        get_params = {
            "vs_currency": self.vs_currency,
            "from": time_from,
            "to": time_to
        }
        r = requests.get(api_url, params=get_params)
        r.raise_for_status()
        response_data = json.loads(r.text)
        prices_data = response_data.get('prices')
        start_timestamp, start_price = prices_data[0]
        end_timestamp, end_price = prices_data[-1]
        price_change = 100 * (end_price - start_price)/start_price
        return price_change
    
    def get_activated_mcaps(self):
        api_url = "https://api.coingecko.com/api/v3/simple/price"
        activated_assets = list(self.activated_assets)
        get_params = {
            "ids": ','.join(activated_assets),
            "vs_currencies": self.vs_currency,
            "include_market_cap": "true"
        }
        r = requests.get(api_url, params=get_params)
        r.raise_for_status()
        response_data = json.loads(r.text)
        activated_asset_to_mcap = {}
        for asset_id in response_data:
            mcap = response_data[asset_id]["{}_market_cap".format(self.vs_currency)]
            activated_asset_to_mcap[asset_id] = mcap
        return activated_asset_to_mcap
    
    async def weighting(self, curr_timestamp):
        self.try_deactivate_expired(curr_timestamp)

        # Check if an activation event was triggered in the past H hours
        for asset in self.asset_ids:
            curr_datetime = datetime.utcfromtimestamp(curr_timestamp)
            time_delta_h = timedelta(hours=self.H+1)
            prev_datetime = curr_datetime - time_delta_h
            price_change = self.get_price_change(asset, prev_datetime.timestamp(), curr_datetime.timestamp())
            if price_change > self.T:
                print("{} was activated at {}".format(asset, curr_datetime.isoformat()))
                self.activate_asset(asset, curr_timestamp)

        if len(self.activated_assets) > 0:
            activated_asset_to_mcap = self.get_activated_mcaps()
            denom = sum(activated_asset_to_mcap.values())

            # weights of the  cluster are first calculated proportional to market cap of each “activated” token
            activated_asset_to_weight = {
                asset_id: mcap/denom
                for asset_id, mcap in activated_asset_to_mcap.items()
            }

            # apply f(w) = min(w, X) for each weight
            for asset in activated_asset_to_weight:
                weight = activated_asset_to_weight[asset]
                activated_asset_to_weight[asset] = min(weight, self.X)

            ust_weight = 1. - sum(activated_asset_to_weight.values())
            target_weights = [ust_weight]
            for asset in self.asset_ids:
                weight = 0
                if asset in activated_asset_to_weight:
                    weight = activated_asset_to_weight[asset]
                target_weights.append(weight)
        else:
            target_weights = self.default_weights
        return target_weights
        
    
    async def recompose(self):
        target_weights = await self.weighting(datetime.now().timestamp())
        print(self.asset_ids, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]

        target = []
        for a, t in zip(self.asset_infos, target_weights):
            native = (a == 'uluna') or (a == 'uusd')
            if t > 0:
                target.append(Asset.asset(a, str(t), native=native))

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

        return self.asset_infos, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    recomposition_bot = NextDogeRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1dsqtuf79093unny85pv53230rzcwehlwxyd5hc")
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))