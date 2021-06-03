from ecosystem import Ecosystem
import asyncio
from tests.provide_liquidity_and_staking import test_provide_liquidity_and_staking
from tests.basket_and_collector_ops import test_basket_and_collector_ops
from tests.community_and_airdrop import test_community_and_airdrop
from tests.governance_ops import test_governance_ops
from tests.incentives_ops import test_incentives_ops


async def main():

    ecosystem = Ecosystem(require_gov=True)
    await ecosystem.initialize_base_contracts()
    await ecosystem.initialize_extraneous_contracts()
    await ecosystem.create_basket(
        100,
        [100, 100],
        [100, 100],
        [1, 1],
        {
            "penalty_amt_lo": "0.1",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "0.5",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.05",
            "reward_cutoff": "0.02",
        },
    )

    # tests are dependent on one another...
    await test_provide_liquidity_and_staking(ecosystem)
    await test_basket_and_collector_ops(ecosystem)
    await test_community_and_airdrop(ecosystem)
    await test_governance_ops(ecosystem)
    await test_incentives_ops(ecosystem)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
