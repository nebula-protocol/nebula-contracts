import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = mnemonic = 'buddy monster west choice floor lonely owner castle mix mouse stable jealous question column regular sad print ethics blame cabbage knife drip practice violin'

from api import Asset
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer

REQUIRE_GOV = True


async def pricing_bot():
    oracle = Contract("terra1qey38t4ptytznnel2ty52r42dm6tee63gnurs2") #dummy oracle

    prices = [
        ("terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u", "3.76"),
        ("terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc", "2.27"),
        ("uluna", "0.000000576")
    ]

    await oracle.set_prices(prices=prices)
    cluster = Contract("terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5")
    cluster_state = await cluster.query.cluster_state(
        cluster_contract_address=cluster
    )
    config = await cluster.query.config()
    print(config)

    print("Updated Cluster State: ", cluster_state)
    # print('setting prices')


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
