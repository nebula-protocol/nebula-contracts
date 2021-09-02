from ecosystem import Ecosystem, deployer


async def test_community_and_airdrop(eco: Ecosystem):
    print("Testing community and airdrop")

    await eco.neb_token.transfer(recipient=eco.airdrop, amount="10000000")

    stage = 1
    claim_amount = "1000000"
    proof = [
        "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42",
        "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f",
        "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3",
        "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2",
    ]

    await eco.airdrop.register_merkle_root(
        merkle_root="3b5c044802c4b768492f98fbd5a9253ed3dd97f5ff129de79a179249e2021766"
    )

    neb_initial = (await eco.neb_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]

    await eco.airdrop.claim(stage=stage, amount=claim_amount, proof=proof)

    neb_final = (await eco.neb_token.query.balance(address=deployer.key.acc_address))[
        "balance"
    ]
    assert int(neb_final) - int(neb_initial) == int(claim_amount)

    initial_community_neb_amt = "100000000"
    await eco.neb_token.transfer(
        recipient=eco.community, amount=initial_community_neb_amt
    )

    spend_amt = "10000"

    if eco.require_gov:
        resp = await eco.create_and_execute_poll(
            {"contract": eco.factory, "msg": decommission_cluster}
        )

        result = await eco.create_and_execute_poll(
            {
                "contract": eco.community,
                "msg": eco.community.spend(
                    recipient=deployer.key.acc_address, amount=spend_amt
                ),
            }
        )

        res = result.logs[0].events_by_type
        assert res["from_contract"]["recipient"][0] == deployer.key.acc_address
        assert res["from_contract"]["amount"][0] == spend_amt
