from ecosystem import Ecosystem, deployer
from contract_helpers import Contract


async def test_governance_ops(eco: Ecosystem):
    print("Testing governance ops")
    new_penalty_contract = await Contract.create(
        eco.code_ids["nebula_penalty"],
        penalty_params={
            "penalty_amt_lo": "0.2",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "1",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.05",
            "reward_cutoff": "0.02",
        },
        owner=eco.cluster,
    )

    await eco.create_and_execute_poll(
        {
            "contract": eco.cluster,
            "msg": eco.cluster.update_config(penalty=new_penalty_contract),
        },
        distribute_collector=True,
    )

    initial_neb = (await eco.neb_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]
    await eco.gov.withdraw_voting_rewards()
    final_neb = (await eco.neb_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]
    assert int(final_neb) > int(initial_neb)
