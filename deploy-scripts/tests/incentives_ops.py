from ecosystem import Ecosystem, deployer
from contract_helpers import chain
from base import terra
from api import Asset


def extract_uusd_amt(bank):
    s = next(i for i in str(bank).split(",") if i.endswith("uusd"))
    return int(s[:-4])


async def test_incentives_ops(eco: Ecosystem):
    print("Testing incentives ops")

    orig_bal = int(
        (await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    print(await eco.cluster.query.cluster_state(cluster_contract_address=eco.cluster))
    print(await eco.cluster_pair.query.pool())
    resp = await eco.incentives.arb_cluster_redeem(
        cluster_contract=eco.cluster,
        asset=Asset.asset("uusd", "1000", native=True),
        _send={"uusd": "1000"},
    )
    # from pprint import pprint
    # for log in resp.logs:
    #     pprint(log.events_by_type)

    new_bal = int(
        (await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    assert new_bal > orig_bal

    old_bal = extract_uusd_amt(await terra.bank.balance(deployer.key.acc_address))
    # print(await eco.cluster_pair.query.pool())
    resp = await chain(
        *[
            i.increase_allowance(spender=eco.incentives, amount="5")
            for i in eco.asset_tokens
        ],
        eco.incentives.arb_cluster_mint(
            cluster_contract=eco.cluster,
            assets=[Asset.asset(i, "5") for i in eco.asset_tokens],
        )
    )

    # for log in resp.logs:
    #     pprint(log.events_by_type)
    new_bal = extract_uusd_amt(await terra.bank.balance(deployer.key.acc_address))
    # 20000000 is the fee amount, we should end get some uusd back from arb_cluster_mint
    assert old_bal - new_bal < 20000000

    old_bal = int(
        (await eco.cluster_token.query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    await chain(
        *[
            i.increase_allowance(spender=eco.incentives, amount="5")
            for i in eco.asset_tokens
        ],
        eco.incentives.mint(
            cluster_contract=eco.cluster,
            asset_amounts=[Asset.asset(i, "5") for i in eco.asset_tokens],
        )
    )
    new_bal = int(
        (await eco.cluster_token.query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    assert new_bal > old_bal

    old_bal = int(
        (await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    await chain(
        eco.cluster_token.increase_allowance(spender=eco.incentives, amount="5"),
        eco.incentives.redeem(
            max_tokens="5",
            cluster_contract=eco.cluster,
        ),
    )
    new_bal = int(
        (await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    assert new_bal > old_bal

    await eco.neb_token.send(
        contract=eco.incentives,
        amount="1000",
        msg=eco.incentives.deposit_reward(rewards=[[1, eco.cluster, "1000"]]),
    )

    await eco.incentives.new_penalty_period()
    await eco.incentives.withdraw()
