from ecosystem import Ecosystem


async def test_basket_and_collector_ops(eco: Ecosystem):
    print("Testing basket and collector ops")

    collector_initial = (await eco.basket_token.query.balance(address=eco.collector))[
        "balance"
    ]

    # mint and redeem which should pass the collector some tokens in fees
    await eco.basket.mint(["10000", "10000"])
    await eco.basket.redeem("5000")

    collector_new = (await eco.basket_token.query.balance(address=eco.collector))[
        "balance"
    ]

    # collector should have gotten basket tokens in the form of fees from the reward/redeem
    assert int(collector_new) > int(collector_initial)

    # basket -> uusd
    await eco.collector.convert(asset_token=eco.basket_token)

    # uusd -> neb
    await eco.collector.convert(asset_token=eco.neb_token)

    # should no longer have bsk left
    assert (await eco.basket_token.query.balance(address=eco.collector))[
        "balance"
    ] == "0"
    # should have some neb
    assert int((await eco.neb_token.query.balance(address=eco.collector))["balance"])
