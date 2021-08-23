import os
import asyncio
import yfinance as yf

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

    async def weighting(self):
        trailing_pe_ratios_inverse = [
            1.0 / yf.Ticker(asset_name).info['trailingPE'] 
            for asset_name in self.asset_names
        ]

        denom = sum(trailing_pe_ratios_inverse)

        target_weights = [val / denom for val in trailing_pe_ratios_inverse]

        mAssets = ['m' + name for name in self.asset_names]
        target_assets = [SYM_TO_CONTRACT_TOKEN_TEQ[mAsset] for mAsset in mAssets]

        print('Target assets', self.asset_names)
        print('Target weights', target_weights)
        target_weights = [int(10000 * target_weight) for target_weight in target_weights]

        # mAssets = ['m' + name for name in self.asset_names]
        # target_assets = [SYM_TO_CONTRACT_TOKEN_TEQ[mAsset] for mAsset in mAssets]

        target = []
        for a, t in zip(target_assets, target_weights):
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

SYM_TO_CONTRACT_TOKEN_TEQ = {
    'ANC': "terra15tecrcm27fenchxaqde9f8ws8krfgjnqf2hhcv",
    'MIR': "terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd",
    'mAAPL': "terra1pwd9etdemugqdt92t5d3g98069z0axpz9plnsk",
    'mABNB': "terra1jm4j6k0e2dpug7z0glc87lwvyqh40z74f40n52",
    'mAMC': "terra1wa87zjty4y983yyt604hdnyr8rm9mwz7let8uz",
    'mAMZN': "terra18mjauk9ug8y29q678c2qlee6rkd9aunrpe9q97",
    'mBABA': "terra1uvzz9fchferxpg64pdshnrc49zkxjcj66uppq8",
    'mBTC': "terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p",
    'mCOIN': "terra16e3xu8ly6a622tjykfuwuv80czexece8rz0gs5",
    'mETH': "terra1rxyctpwzqvldalafvry787thslne6asjlwqjhn",
    'mFB': "terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63",
    'mGLXY': "terra17sm265sez3qle769ef4hscx540wem5hvxztpxg",
    'mGME': "terra19y6tdnps3dsd7qc230tk3jplwl9jm27mpcx9af",
    'mGOOGL': "terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524",
    'mGS': "terra199yfqa5092v2udw0k0h9rau9dzel0jkf5kk3km",
    'mIAU': "terra1n7pd3ssr9sqacwx5hekxsmdy86lwlm0fsdvnwe",
    'mMSFT': "terra18aztjeacdfc5s30ms0558cy8lygvam3s4v69jg",
    'mNFLX': "terra1smu8dc2xpa9rfj525n3a3ttgwnacnjgr59smu7",
    'mQQQ': "terra1r20nvsd08yujq29uukva8fek6g32p848kzlkfc",
    'mSLV': "terra1re6mcpu4hgzs5wc77gffsluqauanhpa8g7nmjc",
    'mSPY': "terra1j3l2ul7s8fkaadwdan67hejt7k5nylmxfkwg0w",
    'mTSLA': "terra1k44gg67rnc6av8sn0602876w8we5lu3jp30yec",
    'mTWTR': "terra1897xd8jqjkfpr5496ur8n896gd8fud3shq3t4q",
    'mUSO': "terra1c3nyehgvukzrt5k9lxzzw64d68el6cejyxjqde",
    'mVIXY': "terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9"
 }