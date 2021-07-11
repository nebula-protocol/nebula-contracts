import os
import sys

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = mnemonic = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from constants import CONTRACT_TOKEN_TO_SYM_TEQ, SYM_TO_MASSET_COL, SYM_TO_COINGECKO_ID

from api import Asset
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer
import requests
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects
import json

REQUIRE_GOV = True
TESTING = True


# async def pricing_bot():
#     oracle = Contract("terra1qey38t4ptytznnel2ty52r42dm6tee63gnurs2") #dummy oracle

#     prices = [
#         ("terra10llyp6v3j3her8u3ce66ragytu45kcmd9asj3u", "3.76"),
#         ("terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc", "2.27"),
#         ("uluna", "0.000000576")
#     ]

#     await oracle.set_prices(prices=prices)
#     cluster = Contract("terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5")
#     cluster_state = await cluster.query.cluster_state(
#         cluster_contract_address=cluster
#     )
#     config = await cluster.query.config()
#     print(config)

#     print("Updated Cluster State: ", cluster_state)
#     # print('setting prices')

def get_prices(symbols):

    url = "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd"

    for info, native in symbols:
        import pdb; pdb.set_trace()
        if native:
            # USE COINGECKO
        
            response = requests.get(url.format(SYM_TO_COINGECKO_ID[info]))
            try:
                data = json.loads(response.text)
            except (ConnectionError, Timeout, TooManyRedirects) as e:
                print(e)
        else:
            price = get_graphql_price(info, testing=TESTING)


# Could do something like if token symbol starts with m [check with token query] -> call this endpoint


async def get_graphql_price(address, testing=False):
    # Note to Manav: if testing is true, that means we're using this on testnet with our made up contract
    # for cluster tokens -> have some dictionaries in constants.py that we can use to query the actual Col-4
    # price (of fake token) once we get to that step

    if testing:
        try:
            sym = CONTRACT_TOKEN_TO_SYM_TEQ['address']
            col_address = SYM_TO_MASSET_COL[sym]
        except:
            raise NameError
    else:
        col_address = address

    query = """
    query {{
        asset(token: {0}) {{
            prices {{
                price
            }}
            symbol
            name
        }}
    }}""".format('\"' + col_address + '\"')

    url = 'https://graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    r.raise_for_status()
    asset = json.loads(r.text)['data']['asset']

    price = asset['prices']['price']
    symbol = asset['symbol']
    return symbol, price


cluster_addr = sys.argv[1]
cluster = Contract(cluster_addr)

async def pricing_bot():

    cfg = (await cluster.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])

    cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster_addr)

    contract_addrs = []
    symbols = []
    query_info = []

    # TODO: Native (?)
    for asset in cluster_state["assets"]:
        if list(asset.keys())[0] == 'native_token':
            # query coingecko
            denom = asset["native_token"]["denom"]
            contract_addrs.append(denom)
            symbols.append(denom)
            query_info.append([denom, True])
        else:
            addr = asset["token"]["contract_addr"]
            token_info = await Contract(addr).query.token_info()
            contract_addrs.append(addr)
            symbols.append(token_info["symbol"])
            query_info.append([token_info["symbol"], False])

    while True:
        # TODO: FIX THIS
        price_data = get_prices(query_info)
        set_prices_data = []
        for i in range(len(contract_addrs)):
            set_prices_data.append(
                [contract_addrs[i], str(price_data[symbols[i][0]]["price"])]
            )

        await oracle.set_prices(prices=set_prices_data)
        basket_state = await cluster.query.basket_state(
            basket_contract_address=cluster_addr
        )
        print("new prices", price_data)
        await asyncio.sleep(30)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
