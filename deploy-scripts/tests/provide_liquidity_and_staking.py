from ecosystem_testing import Ecosystem
from contract_helpers import chain, deployer, dict_to_b64
from api import Asset
import asyncio


async def test_provide_liquidity_and_staking(eco: Ecosystem):
    print("Testing providing liquidity and staking")

    inc_allowance = eco.cluster_token.increase_allowance(
        spender=eco.cluster_pair, amount="50"
    )
    provide_liquidity = eco.cluster_pair.provide_liquidity(
        assets=[
            Asset.asset(eco.cluster_token, amount="50"),
            Asset.asset("uusd", amount="50", native=True),
        ],
        _send={"uusd": "50"},
    )

    resp = await chain(inc_allowance, provide_liquidity)
    n_lp_tokens = resp.logs[1].events_by_type["from_contract"]["amount"][-1]

    neb_before_staking = (
        await eco.neb_token.query.balance(address=deployer.key.acc_address)
    )["balance"]

    bond_msg = {
        "bond": {
            "asset_token": eco.cluster_token.address
        }
    }

    await eco.lp_token.send(
        amount=n_lp_tokens,
        contract=eco.staking,
        msg=dict_to_b64(bond_msg),
    )

    await asyncio.sleep(1)
    await eco.factory.distribute()

    await eco.staking.unbond(asset_token=eco.cluster_token, amount="10")

    await eco.staking.withdraw(asset_token=eco.cluster_token)

    neb_after_staking = (
        await eco.neb_token.query.balance(address=deployer.key.acc_address)
    )["balance"]

    # should receive neb rewards for the second
    assert int(neb_after_staking) > int(neb_before_staking)

    # should have exactly 10 in balance after bond / unbond
    assert (await eco.lp_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ] == "10"
