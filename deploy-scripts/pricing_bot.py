import os
import sys

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = mnemonic = 'race such cash farm action hockey false recall protect digital sort remind squeeze home area balance focus chunk rare coyote excess rhythm swarm civil'

from constants import CONTRACT_TOKEN_TO_SYM_TEQ, SYM_TO_MASSET_COL, SYM_TO_COINGECKO_ID

from api import Asset
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer
import requests
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects
import json

import numpy as np

REQUIRE_GOV = True
TESTING = True

async def get_graphql_price(address, testing=False):
    # Note to Manav: if testing is true, that means we're using this on testnet with our made up contract
    # for cluster tokens -> have some dictionaries in constants.py that we can use to query the actual Col-4
    # price (of fake token) once we get to that step

    if testing:
        try:
            sym = CONTRACT_TOKEN_TO_SYM_TEQ[address]
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
    return price

async def get_prices(infos):

    url = "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd"
    prices = []
    for info, mirrored in infos:
        if mirrored:

            # Query for mAssets
            price = await get_graphql_price(info, testing=TESTING)
            prices.append(price)

        else:
            # USE COINGECKO
            cg_id = SYM_TO_COINGECKO_ID[info]
        
            response = requests.get(url.format(cg_id))
            try:
                data = json.loads(response.text)[cg_id]['usd']

                # Should encompass uluna + uust (test uust)
                if info.islower():
                    data = data
                prices.append(data)

            except (ConnectionError, Timeout, TooManyRedirects) as e:
                print(e)
            
    print(prices)
    return prices



cluster_addr = sys.argv[1]
cluster = Contract(cluster_addr)

async def pricing_bot():

    cfg = (await cluster.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])

    await oracle.set_prices(prices=[('uust','0.0000001')])

    cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster_addr)

    contract_addrs = []
    symbols = []
    query_info = []

    # TODO: Native (?)c
    for asset in cluster_state["assets"]:
        if list(asset.keys())[0] == 'native_token':
            # query coingecko
            denom = asset["native_token"]["denom"]
            contract_addrs.append(denom)
            symbols.append(denom)
            query_info.append([denom, False])
        else:
            addr = asset["token"]["contract_addr"]
            token_info = await Contract(addr).query.token_info()
            symbol = token_info["symbol"]
            contract_addrs.append(addr)
            symbols.append(symbol)
            if symbol[0] == 'm':
                # Use address for mirrored assets
                query_info.append([addr, True])
            else:
                # Use symbol mapping for CoinGecko
                query_info.append([symbol, False])
            
    while True:
        # TODO: FIX THIS
        price_data = await get_prices(query_info)
        set_prices_data = []

        for i in range(len(contract_addrs)):
            set_prices_data.append(
                [
                    contract_addrs[i], np.format_float_positional(
                        np.round(float(price_data[i]), 18),
                        trim='0'
                    )
                ]
            )

        await oracle.set_prices(prices=set_prices_data)
        cluster_state = await cluster.query.cluster_state(
            cluster_contract_address=cluster_addr
        )
        print("new prices", price_data)
        await asyncio.sleep(15)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
