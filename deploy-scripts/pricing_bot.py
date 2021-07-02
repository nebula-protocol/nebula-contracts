import os

os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer

REQUIRE_GOV = True


async def pricing_bot():
    oracle = Contract("terra16hy3a4fqjte0yjduzjeurzsvct885xqa30u9zj") #dummy oracle

    # print("cluster", cluster)

    prices = [
        ("terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u", "3.76"),
        ("terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc", "2.27"),
        ("uluna", "0.000000576")
    ]

    oracle.set_prices(prices=prices)
    price = await oracle.query.price(
            base_asset="uluna", quote_asset="uusd"
        )
    print(price)
    # print('setting prices')


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
