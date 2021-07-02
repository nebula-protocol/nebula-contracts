from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain, terra
import asyncio
from base import deployer


async def main():
    print(await terra.bank.balance(deployer.key.acc_address))
    ecosystem = Ecosystem()
    asset_tokens = ["uusd", "uluna"]

    await ecosystem.initialize_base_contracts()
    await ecosystem.initialize_extraneous_contracts()

    code_ids = ecosystem.code_ids

    penalty_params = {
        "penalty_amt_lo": "0.1",
        "penalty_cutoff_lo": "0.01",
        "penalty_amt_hi": "0.5",
        "penalty_cutoff_hi": "0.1",
        "reward_amt": "0.05",
        "reward_cutoff": "0.02",
    }
    target_weights = [1, 1]

    penalty_contract = await Contract.create(
        code_ids["nebula_penalty"],
        penalty_params=penalty_params,
        owner=ecosystem.factory,
    )

    oracle = await Contract.create(
        code_ids["nebula_dummy_oracle"],
        terraswap_factory=ecosystem.terraswap_factory,
        base_denom="uusd",
    )

    resp = await ecosystem.factory.create_cluster(
        name="CLUSTER",
        symbol="BSK",
        params={
            "name": "CLUSTER",
            "symbol": "BSK",
            "penalty": penalty_contract,
            "target": target_weights,
            "assets": [Asset.native_asset_info(i) for i in asset_tokens],
            "pricing_oracle": oracle,
            "composition_oracle": oracle,
        },
    )

    logs = resp.logs[0].events_by_type

    instantiation_logs = logs["instantiate_contract"]
    addresses = instantiation_logs["contract_address"]

    cluster_token = Contract(addresses[2])
    cluster_pair = Contract(addresses[1])
    lp_token = Contract(addresses[0])

    cluster = Contract(
        addresses[3],
    )
    cfg = await cluster.query.config()
    print(cfg)
    oracle = Contract(cfg["config"]["pricing_oracle"])
    await oracle.set_prices(prices=[["uusd", "1"], ["uluna", "1"]])
    asset_amounts = [Asset.asset(i, "100000", native=True) for i in asset_tokens]

    await cluster.mint(
        asset_amounts=asset_amounts,
        min_tokens="100000",
        _send={i: "100000" for i in asset_tokens},
    )

    await ecosystem.incentives.mint(
        cluster_contract=cluster,
        asset_amounts=asset_amounts,
        _send={i: "100000" for i in asset_tokens},
    )

    await chain(
        cluster_token.increase_allowance(spender=ecosystem.incentives, amount="150000"),
        ecosystem.incentives.redeem(
            cluster_contract=cluster, max_tokens="150000", asset_amounts=asset_amounts
        ),
    )

    await chain(
        cluster_token.increase_allowance(amount="50000", spender=cluster_pair),
        cluster_pair.provide_liquidity(
            assets=[
                Asset.asset(cluster_token, amount="50000"),
                Asset.asset("uusd", amount="50000", native=True),
            ],
            _send={"uusd": "50000"},
        ),
    )

    await ecosystem.incentives.arb_cluster_mint(
        cluster_contract=cluster,
        assets=asset_amounts,
        _send={i: "100000" for i in asset_tokens},
    )

    await ecosystem.incentives.arb_cluster_redeem(
        cluster_contract=cluster,
        asset=Asset.asset("uusd", amount="100000", native=True),
        _send={"uusd": "100000"},
    )


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
