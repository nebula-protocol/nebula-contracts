import requests
import json
import pandas as pd

async def mirror_history_query(address, tick, from_stamp, to_stamp):
    query = """
    query {{
        asset(token: {0}) {{
            prices {{
                history(interval: {1}, from: {2}, to: {3}) {{
                    timestamp
                    price
                }}
            }}
            statistic {{
                marketCap
            }}
            symbol
            name
        }}
    }}""".format('\"' + address + '\"', str(tick), str(from_stamp), str(to_stamp))

    # url = 'https://graph.mirror.finance/graphql'

    # TODO: CHANGE FROM TESTNET
    url = 'https://tequila-graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    try:
        r.raise_for_status()
        asset = json.loads(r.text)['data']['asset']

        prices = asset['prices']['history']
        symbol = asset['symbol']
        mcap = asset['statistic']['marketCap']
        latest_timestamp = max([p['timestamp'] for p in prices])
        closes = [p['price'] for p in prices]
        return symbol, latest_timestamp, closes, mcap
    except:
        return None, None, None, None
    

async def get_all_mirror_assets():
    query = """
    query {
        assets {
            token
        }
    }"""
    
    url = 'https://tequila-graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query}, headers=None)
    r.raise_for_status()
    assets = json.loads(r.text)['data']['assets']
    addresses = [a['token'] for a in assets]

    return addresses