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
            }},
            statistic {{
                marketCap
            }},
            symbol,
            name
        }}
    }}""".format('\"' + address + '\"', str(tick), str(from_stamp), str(to_stamp))

    url = 'https://graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    r.raise_for_status()
    asset = json.loads(r.text)['data']['asset']

    prices = asset['prices']['history']
    symbol = asset['symbol']
    mcap = asset['statistic']['marketCap']
    latest_timestamp = max([p['timestamp'] for p in prices])
    closes = [p['price'] for p in prices]

    return symbol, latest_timestamp, closes, mcap

async def get_all_mirror_assets():
    query = """
    query {{
        assets {{
            token,
            name,
            symbol
        }}
    }}"""

    # TODO: Figure out why this isn't working

    headers = {'Accept-Encoding': 'gzip, deflate, br',
               'Content-Type': 'application/json',
               'Accept': 'application/json',
               'Connection': 'keep-alive',
               'DNT': '1',
               'Origin': 'https://graph.mirror.finance',
               'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36'}
    url = 'https://graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query}, headers=headers)
    r.raise_for_status()
    assets = json.loads(r.text)['data']['assets']

    addresses = [a['token'] for a in assets]

    return addresses