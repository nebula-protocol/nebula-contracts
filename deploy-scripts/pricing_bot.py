import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = mnemonic = 'lottery horn blast wealth cruise border opinion upgrade common gauge grocery evil canal lizard sad mad submit degree brave margin age lunar squirrel diet'

from api import Asset
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer

REQUIRE_GOV = True


async def pricing_bot():
    oracle = Contract("terra1l3k2gnmcy8wx69ycrmcetrxmpv9kye6htxrxqh") #dummy oracle

    prices = [
        ("terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u", "3.76"),
        ("terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc", "2.27"),
        ("uluna", "0.000000576")
    ]

    oracle.set_prices(prices=prices)
    cluster = Contract("terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5")
    cluster_state = await cluster.query.cluster_state(
        cluster_contract_address=cluster
    )

    print("Updated Cluster State: ", cluster_state)
    # print('setting prices')


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
