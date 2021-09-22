import os
import asyncio
import yfinance as yf
from .pricing import get_query_info, get_prices
from .graphql_querier import SYM_TO_CONTRACT_TOKEN_BOMBAY_11

os.environ["MNEMONIC"] = mnemonic = 'soda buffalo melt legal zebra claw taxi peace fashion service drastic special coach state rare harsh business bulb tissue illness juice steel screen chef'
os.environ["USE_TEQUILA"] = "1"

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
        mAssets = ['m' + name for name in self.asset_names]
        self.target_assets = [SYM_TO_CONTRACT_TOKEN_BOMBAY_11[mAsset] for mAsset in mAssets]

    async def weighting(self):
        _, _, query_info = await get_query_info(self.target_assets)
        trailing_pe_ratios_inverse = [
            1.0 / yf.Ticker(asset_name).info['trailingPE'] 
            for asset_name in self.asset_names
        ]

        denom = sum(trailing_pe_ratios_inverse)

        target_weights = [val / denom for val in trailing_pe_ratios_inverse]

        mAssets = ['m' + name for name in self.asset_names]
        prices = await get_prices(query_info)

        print('Target assets', self.asset_names)
        print('Target weights', target_weights)

        target = []
        for a, t, p in zip(self.target_assets, target_weights, prices):
            native = (a == 'uluna')
            print(t)
            tw = str(int(100000000 * t / float(p)))
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

async def run_retarget_periodically(cluster_contract, interval):
    start_time = time.time()
    
    retarget_bot = FABMANGRecomposer(cluster_contract)

    while True:
        await asyncio.gather(
            asyncio.sleep(interval),
            retarget_bot.recompose(),
        )

if __name__ == "__main__":
    cluster_contract = Contract("terra1hpx6dtjt6lxq46t3trwqnlsvhusslmuu84z57m")
    interval = SECONDS_PER_DAY
    asyncio.get_event_loop().run_until_complete(run_retarget_periodically(cluster_contract, interval))