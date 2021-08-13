import os
import asyncio
import yfinance as yf
from pricing import get_query_info, get_prices
from graphql_querier import SYM_TO_CONTRACT_TOKEN_TEQ

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
        self.target_assets = [SYM_TO_CONTRACT_TOKEN_TEQ[mAsset] for mAsset in mAssets]

    async def weighting(self):
        _, _, query_info = await get_query_info(self.target_assets)
        trailing_pe_ratios_inverse = [
            1.0 / yf.Ticker(asset_name).info['trailingPE'] 
            for asset_name in self.asset_names
        ]

        denom = sum(trailing_pe_ratios_inverse)

        target_weights = [val / denom for val in trailing_pe_ratios_inverse]

        mAssets = ['m' + name for name in self.asset_names]
        target_assets = [SYM_TO_CONTRACT_TOKEN_BOMBAY[mAsset] for mAsset in mAssets]
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

SYM_TO_CONTRACT_TOKEN_BOMBAY = {
  'AAVE': 'terra1exw6sae4wyq8rt56hxdggzmgmqsuukr26u4aj8',
  'ANC': 'terra1mst8t7guwkku9rqhre4lxtkfkz3epr45wt8h0m',
  'AUDIO': 'terra1t89u7cfrp9r4a8msmxz4z3esn5g5z8ga2qsec6',
  'AXS': 'terra1w07h8u34an2jcfsegjc80edunngf3ey6xdz456',
  'COMP': 'terra10af2zy62wanc6cs3n66cplmpepvf6qnetuydz2',
  'CREAM': 'terra1a7g946jyjhn8h7gscda7sd68kn9k4whkxq0ddn',
  'ENJ': 'terra14vxe68djqpmzspvkaj9fjxc8fu6qmt34wmm6xc',
  'MANA': 'terra1lc6czeag9zaaqk04y5ynfkxu723n7t56kg2a9r',
  'MKR': 'terra1lflvesvarcfu53gd9cgkv3juyrz79cnk7yw6am',
  'SAND': 'terra1q3smy9j5qjplyas4l3tgyj72qtq9fvysff4msa',
  'MIR': 'terra159nvmamkrj0hw5e0e0lp4vzh6py0ev765jgl58',
  'mAAPL': 'terra1qkk30fyqn27fz0a0h7alx6h73pjhur0afxlamy',
  'mABNB': 'terra10x0h5r0t9hdwamdxehapjnj67p4f8nx38pxuzx',
  'mAMC': 'terra1kvjetgk5arnsyn4t4cer8ppttdlymcn35awdc7',
  'mAMZN': 'terra1gua38jnfldhrqw6xgshwe4phkdyuasfnv5jfyu',
  'mBABA': 'terra1fczn32j5zt0p9u9eytxa7cdzvhu7yll06lzvl3',
  'mBTC': 'terra14js9dgr87dxepx2gczkudxj69xudf2npnw87f9',
  'mCOIN': 'terra1ml4dh06egs4ezjhq5r50ku3zc8086yfsvtreyl',
  'mETH': 'terra1p8qzs0glqkfx6e08alr5c66vnlscl09df2wmwa',
  'mFB': 'terra1ucsa089wnu7u6qe05ujp4vzvf73u9aq3u89ytn',
  'mGLXY': 'terra1et7mmctaffrg0sczfaxkqwksd43wt5fvjhyffd',
  'mGME': 'terra1ud984ssduc53q6z90raydwe4akch98q0ksr5ry',
  'mGOOGL': 'terra1ym8kp806plgum787fxpukj6z8tg90eslklppfq',
  'mGS': 'terra1lj8x2s06vmfherel08qptvv22wqld0z3ytmzcf',
  'mIAU': 'terra1xx5ndkhe477sa267fc6mryq7jekk6aczep6mqh',
  'mMSFT': 'terra1kqqqhtqsu9h4c93rlmteg2zuhc0z53ewlwt8vq',
  'mNFLX': 'terra1r27h7zpchq40r54x64568yep3x8j93lr5u2g24',
  'mQQQ': 'terra1wxjq2lsxvhq90z0muv4nkcddjt23t89vh4s4d6',
  'mSLV': 'terra1gytrpc5972ed3gthmupmc6wxyayx4wtvmzq8cy',
  'mTSLA': 'terra1lrtvldvfkxx47releuk266numcg2k29y7t8t2n',
  'mTWTR': 'terra1a4jtyzta9zr3df8w2f5d8zr44ws0dm58cznsca',
  'mUSO': 'terra1tpsls0lzyh2fkznhjyes56upgk5g4z0sw3hgdn',
  'mVIXY': 'terra1yvplcammukw0d5583jw4payn0veqtgfumqvjk0'
 }
