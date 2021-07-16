import os
import asyncio
import yfinance as yf

from graphql_querier import SYM_TO_CONTRACT_TOKEN_TEQ

os.environ["MNEMONIC"] = mnemonic = 'idea salute sniff electric lecture table flag oblige pyramid light ocean heart web ramp save fiscal sting course uncle deputy way field vacant genius'

os.environ["USE_TEQUILA"] = "1"

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.key.mnemonic import MnemonicKey

from api import Asset
from contract_helpers import Contract, ClusterContract, terra
import time

SECONDS_PER_DAY = 24 * 60 * 60

"""
Recomposes according to P/E ratios of FAB MANG mAssets.
"""
class FABMANGRecomposer:
    def __init__(self, cluster_contract):
        self.cluster_contract = cluster_contract
        self.asset_names = ["FB", "AAPL", "BABA", "MSFT", "AMZN", "NFLX", "GOOGL"]

    async def weighting(self):
        trailing_pe_ratios_inverse = [
            1.0 / yf.Ticker(asset_name).info['trailingPE'] 
            for asset_name in self.asset_names
        ]

        denom = sum(trailing_pe_ratios_inverse)

        target_weights = [val / denom for val in trailing_pe_ratios_inverse]

        return self.asset_names, target_weights
    
    async def recompose(self):

        asset_names, target_weights = await self.weighting()
        print('Target assets', asset_names)
        print('Target weights', target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]

        mAssets = ['m' + name for name in self.asset_names]
        target_assets = [SYM_TO_CONTRACT_TOKEN_TEQ[mAsset] for mAsset in mAssets]

        await self.cluster_contract.reset_target(
            assets=[Asset.asset_info(a) for a in target_assets],
            target=target_weights
        )

        # target = await self.cluster_contract.query.target()
        # cluster = Contract("terra1wa7frpp078hnqnlvevmqjyswvnswp4psmkjred")
        # cluster_state = await self.cluster_contract.query.cluster_state(
        #     cluster_contract_address=cluster
        # )

        # print("Updated Target: " , target)
        # print("Updated Cluster State: ", cluster_state)
        return target_assets, target_weights

async def run_recomposition_periodically(cluster_contract, interval):
    start_time = time.time()
    
    recomposition_bot = FABMANGRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            recomposition_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra15qcvpgnwecl82rljfupcnl4ek9gqej4mcpy4xf") #TODO: UPDATE
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_recomposition_periodically(cluster_contract, interval))