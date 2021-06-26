from ecosystem import Ecosystem
import asyncio
from tests.provide_liquidity_and_staking import test_provide_liquidity_and_staking
from tests.cluster_and_collector_ops import test_cluster_and_collector_ops
from tests.community_and_airdrop import test_community_and_airdrop
from tests.governance_ops import test_governance_ops
from tests.incentives_ops import test_incentives_ops

import sys
import time
import random

async def recompose():
    print("Insert recompose query code here")


async def run_recomposition_periodically(interval, periodic_function):
    start_time = time.time()
    while True:
        print(round(time.time() - start_time, 1), "Recomposition")
        await asyncio.gather(
            asyncio.sleep(interval),
            periodic_function(),
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

    await run_recomposition_periodically(5, recompose)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
