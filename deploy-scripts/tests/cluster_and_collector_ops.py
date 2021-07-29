from ecosystem import Ecosystem, deployer


async def test_cluster_and_collector_ops(eco: Ecosystem):
    print("Testing cluster and collector ops")

    collector_initial = (await eco.cluster_token.query.balance(address=eco.collector))[
        "balance"
    ]

    await eco.dummy_oracle.set_prices(prices=list(zip(eco.asset_tokens, eco.asset_prices)))
    # mint and redeem which should pass the collector some tokens in fees
    await eco.cluster.mint(["10000", "10000"])
    balance = (await eco.cluster_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]
    print('Balance after mint', balance)

    await eco.cluster.redeem("5000")
    balance = (await eco.cluster_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]
    print('Balance after redeem', balance)

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

async def test_decommission_cluster(eco: Ecosystem):
    print("Testing death of a cluster")

    decommission_cluster = eco.factory.decommission_cluster(
        cluster_contract=eco.cluster,
        cluster_token=eco.cluster_token,
    )

    if eco.require_gov:
        resp = await eco.create_and_execute_poll(
            {"contract": eco.factory, "msg": decommission_cluster}
        )
    else:
        resp = await decommission_cluster

    logs = resp.logs[0].events_by_type

    cluster_state = await eco.cluster.query.cluster_state(
        cluster_contract_address=eco.cluster.address
    )
    print(cluster_state)
    try:
        await eco.cluster.mint(["10000", "10000"])
        raise ValueError
    except:
        print('Mint failed as expected') 
    
    await eco.cluster.redeem("2000")

    cluster_list = await eco.factory.query.cluster_list()
    assert (
            not cluster_list["contract_infos"][0][1]
        )

    print(cluster_list)

    print("Integration test completed")
