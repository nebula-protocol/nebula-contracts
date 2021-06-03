from ecosystem import Ecosystem
from api import Asset


async def test_incentives_ops(eco: Ecosystem):
    print("Testing incentives ops")

    await eco.incentives.arb_cluster_redeem(
        basket_contract=eco.basket,
        asset=Asset.asset("uusd", "10", native=True),
        _send={"uusd": "10"},
    )

    await eco.neb_token.send(
        contract=eco.incentives,
        amount="1000",
        msg=eco.incentives.deposit_reward(rewards=[[1, eco.basket, "1000"]]),
    )

    await eco.incentives.new_penalty_period()
    await eco.incentives.withdraw()
