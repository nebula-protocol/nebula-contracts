import asyncio
from contract_helpers import Contract, store_contracts, deployer, Asset, chain


async def main():
    code_ids = await store_contracts()

    neb_token = await Contract.create(
        code_ids["terraswap_token"],
        name="Nebula Token",
        symbol="NEB",
        decimals=6,
        initial_balances=[
            {
                "address": deployer.key.acc_address,
                "amount": "1000000000000",
            },
        ],
    )

    terraswap_factory = await Contract.create(
        code_ids["terraswap_factory"],
        pair_code_id=int(code_ids["terraswap_pair"]),
        token_code_id=int(code_ids["terraswap_token"]),
    )

    resp = await terraswap_factory.create_pair(
        asset_infos=[Asset.cw20_asset_info(neb_token), Asset.native_asset_info("uusd")]
    )

    logs = resp.logs[0].events_by_type
    pair = logs["from_contract"]["pair_contract_addr"][0]

    pair = Contract(pair)
    await chain(
        neb_token.increase_allowance(amount="10", spender=pair),
        pair.provide_liquidity(
            assets=[
                Asset.asset(neb_token, amount="10"),
                Asset.asset("uusd", amount="10", native=True),
            ],
            _send={"uusd": "10"},
        ),
    )

    await pair.swap(
        offer_asset=Asset.asset("uusd", amount="10000", native=True),
        _send={"uusd": "10000"}
    )

    # pool now has zero neb tokens but positive USD ?!?!
    # terraswap pool is now bricked
    print(await pair.query.pool())

    # subsequent transactions all run out gas, presumably from some uncaught error
    # in the terraswap contract

    # swap neb -> UUSD
    await neb_token.send(
        amount="1000",
        contract=pair,
        msg=pair.swap(offer_asset=Asset.asset(neb_token, amount="1000")),
    )

    # provide liquidity
    await chain(
        neb_token.increase_allowance(amount="10", spender=pair),
        pair.provide_liquidity(
            assets=[
                Asset.asset(neb_token, amount="10"),
                Asset.asset("uusd", amount="10", native=True),
            ],
            _send={"uusd": "10"},
        ),
    )

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
