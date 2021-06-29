from ecosystem import Ecosystem
import asyncio
from tests.provide_liquidity_and_staking import test_provide_liquidity_and_staking
from tests.cluster_and_collector_ops import test_cluster_and_collector_ops
from tests.community_and_airdrop import test_community_and_airdrop
from tests.governance_ops import test_governance_ops
from tests.incentives_ops import test_incentives_ops
from recomposition_bot import RecompositionBot
from bot_code.bullish_cross import BullishCrossRecomposer
from api import Asset

import sys
import time
import random

async def recompose(ecosystem, rec_bot, asset_names):
    new_assets, new_target = await rec_bot.recompose()

    await ecosystem.cluster.reset_target(
        assets=[Asset.cw20_asset_info(a) for a in new_assets],
        target=new_target
    )


async def run_recomposition_periodically(interval, ecosystem):
    start_time = time.time()

    # Change bot here
    # asset_addresses = ['mFB', 'mTSLA', 'mGOOGL']
    asset_addresses = ['terra1mqsjugsugfprn3cvgxsrr8akkvdxv2pzc74us7', 
                       'terra14y5affaarufk3uscy2vr6pe6w6zqf2wpjzn5sh', 
                       'terra1h8arz2k547uvmpxctuwush3jzc8fun4s96qgwt']
    rec_bot = BullishCrossRecomposer(asset_addresses, use_test_data=False)
    
    while True:
        print(round(time.time() - start_time, 1), "Recomposition")
        await asyncio.gather(
            asyncio.sleep(interval),
            recompose(ecosystem, rec_bot, asset_addresses),
        )


async def main():

    ecosystem = Ecosystem(require_gov=True)
    await ecosystem.initialize_base_contracts()
    await ecosystem.initialize_extraneous_contracts()
    await ecosystem.create_cluster(
        100,
        [100, 100, 100],
        [100, 100, 100],
        [1, 1, 1],
        {
            "penalty_amt_lo": "0.1",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "0.5",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.05",
            "reward_cutoff": "0.02",
        },
        asset_names=['mFB', 'mTSLA', 'mGOOGL']
    )

    await run_recomposition_periodically(5, ecosystem)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
