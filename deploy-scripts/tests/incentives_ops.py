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

    await eco.incentives.arb_cluster_redeem(
        basket_contract=eco.basket,
        asset=Asset.asset("uusd", "10", native=True),
        _send={"uusd": "10"},
    )

    new_bal = int(
        (await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))[
            "balance"
        ]
    )
    assert new_bal > orig_bal

    old_bal = extract_uusd_amt(await terra.bank.balance(deployer.key.acc_address))

    await chain(
        *[
            i.increase_allowance(spender=eco.incentives, amount="5")
            for i in eco.asset_tokens
        ],
        eco.incentives.arb_cluster_mint(
            basket_contract=eco.basket,
            assets=[Asset.asset(i, "5") for i in eco.asset_tokens],
        )
    )
    new_bal = extract_uusd_amt(await terra.bank.balance(deployer.key.acc_address))
    # 20000000 is the fee amount, we should end get some uusd back from arb_cluster_mint
    assert old_bal - new_bal < 20000000

    old_bal = int((await eco.basket_token.query.balance(address=deployer.key.acc_address))["balance"])
    await chain(
        *[
            i.increase_allowance(spender=eco.incentives, amount="5")
            for i in eco.asset_tokens
        ],
        eco.incentives.mint(
            basket_contract=eco.basket,
            asset_amounts=[Asset.asset(i, "5") for i in eco.asset_tokens],
        )
    )
    new_bal = int((await eco.basket_token.query.balance(address=deployer.key.acc_address))["balance"])
    assert new_bal > old_bal

    old_bal = int((await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))["balance"])
    await chain(
        eco.basket_token.increase_allowance(spender=eco.incentives, amount="5"),
        eco.incentives.redeem(
            max_tokens="5",
            basket_contract=eco.basket,
        )
    )
    new_bal = int((await eco.asset_tokens[0].query.balance(address=deployer.key.acc_address))["balance"])
    assert new_bal > old_bal

    await eco.neb_token.send(
        contract=eco.incentives,
        amount="1000",
        msg=eco.incentives.deposit_reward(rewards=[[1, eco.basket, "1000"]]),
    )

    await eco.incentives.new_penalty_period()
    await eco.incentives.withdraw()
