from ecosystem import Ecosystem


async def test_cluster_and_collector_ops(eco: Ecosystem):
    print("Testing cluster and collector ops")

    collector_initial = (await eco.cluster_token.query.balance(address=eco.collector))[
        "balance"
    ]

    await eco.dummy_oracle.set_prices(prices=list(zip(eco.asset_tokens, eco.asset_prices)))
    # mint and redeem which should pass the collector some tokens in fees
    await eco.cluster.mint(["10000", "10000"])
    await eco.cluster.redeem("5000")

    collector_new = (await eco.cluster_token.query.balance(address=eco.collector))[
        "balance"
    ]

    # collector should have gotten cluster tokens in the form of fees from the reward/redeem
    assert int(collector_new) > int(collector_initial)

    # cluster -> uusd
    await eco.collector.convert(asset_token=eco.cluster_token)

    # uusd -> neb
    await eco.collector.convert(asset_token=eco.neb_token)

    # should no longer have bsk left
    assert (await eco.cluster_token.query.balance(address=eco.collector))[
        "balance"
    ] == "0"
    # should have some neb
    assert int((await eco.neb_token.query.balance(address=eco.collector))["balance"])
