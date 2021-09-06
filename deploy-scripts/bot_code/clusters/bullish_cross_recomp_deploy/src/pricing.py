import os
import sys

os.environ["USE_BOMBAY"] = "1"
os.environ["MNEMONIC"] = mnemonic = 'buddy monster west choice floor lonely owner castle mix mouse stable jealous question column regular sad print ethics blame cabbage knife drip practice violin'

from graphql_querier import CONTRACT_TOKEN_TO_SYM_BOMBAY, SYM_TO_MASSET_COL, SYM_TO_COINGECKO_ID

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
            sym = CONTRACT_TOKEN_TO_SYM_BOMBAY[address]
            col_address = SYM_TO_MASSET_COL[sym]
        except:
            raise NameError
    else:
        col_address = address

    query = """
    query {{
        asset(token: {0}) {{
            prices {{
                oraclePrice
            }}
            symbol
            name
        }}
    }}""".format('\"' + col_address + '\"')

    url = 'https://graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    r.raise_for_status()
    asset = json.loads(r.text)['data']['asset']

    price = asset['prices']['oraclePrice']
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
                prices.append(str(data))

            except (ConnectionError, Timeout, TooManyRedirects) as e:
                print(e)
            
    print(prices)
    return prices

async def get_query_info(to_query=None):
    contract_addrs = []
    symbols = []
    query_info = []

    if to_query is None:
        for native in ['uusd', 'uluna']:
            contract_addrs.append(native)
            query_info.append([native, False])

        for addr in list(CONTRACT_TOKEN_TO_SYM_BOMBAY.keys()):
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

    else:
        for name in to_query:
            native = bool(name in ['uusd', 'uluna'])
            contract_addrs.append(name)

            if native:
                query_info.append([name, False])
            else:
                token_info = await Contract(name).query.token_info()
                symbol = token_info["symbol"]
                symbols.append(symbol)
                if symbol[0] == 'm':
                    # Use address for mirrored assets
                    query_info.append([name, True])
                else:
                    # Use symbol mapping for CoinGecko
                    query_info.append([symbol, False])


    return contract_addrs, symbols, query_info

async def set_prices(oracle, contract_addrs, query_info):
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

    # print(set_prices_data)

    await oracle.set_prices(prices=set_prices_data)

async def pricing_bot():
    oracle = Contract("terra1ajjdnwvmhgc36p75apzrzkh2ekd8af3hqlzeka")

    contract_addrs, symbols, query_info = get_query_info()
            
    while True:
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

        print(set_prices_data)

        await oracle.set_prices(prices=set_prices_data)
        print("new prices", price_data)
        await asyncio.sleep(15)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(pricing_bot())
