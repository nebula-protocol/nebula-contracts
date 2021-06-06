import os

os.environ["USE_TEQUILA"] = "1"

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, BasketContract
import asyncio


async def main():

    ecosystem = Ecosystem(require_gov=False)

    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    asset_tokens = [
        Contract("terra14gq9wj0tt6vu0m4ec2tkkv4ln3qrtl58lgdl2c"),  # mFB
        Contract("terra16vfxm98rxlc8erj4g0sj5932dvylgmdufnugk0"),  # mAAPL
        Contract("terra1djnlav60utj06kk9dl7defsv8xql5qpryzvm3h"),  # mNFLX
        Contract("terra1qg9ugndl25567u03jrr79xur2yk9d632fke3h2"),  # mGOOGL
    ]

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
    target_weights = [1, 1, 1, 1]

    penalty_contract = await Contract.create(
        code_ids["basket_penalty"],
        penalty_params=penalty_params,
        owner=ecosystem.factory,
    )

    oracle = await Contract.create(
        code_ids["terraswap_oracle"],
        terraswap_factory=ecosystem.terraswap_factory,
        base_denom="uusd",
    )

    resp = await ecosystem.factory.create_cluster(
        name="BASKET",
        symbol="BSK",
        params={
            "name": "BASKET",
            "symbol": "BSK",
            "penalty": penalty_contract,
            "target": target_weights,
            "assets": [Asset.cw20_asset_info(i) for i in asset_tokens],
            "pricing_oracle": oracle,
            "composition_oracle": oracle,
        },
    )

    logs = resp.logs[0].events_by_type

    instantiation_logs = logs["instantiate_contract"]
    addresses = instantiation_logs["contract_address"]

    basket_token = Contract(addresses[2])
    basket_pair = Contract(addresses[1])
    lp_token = Contract(addresses[0])

    basket = BasketContract(
        addresses[3],
        basket_token,
        asset_tokens,
    )

    resp = await basket.query.basket_state(basket_contract_address=basket)
    print(resp)

    print("basket", basket)
    print("assets", asset_tokens)
    print("oracle", oracle)
    print("ecosystem", ecosystem.__dict__)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
