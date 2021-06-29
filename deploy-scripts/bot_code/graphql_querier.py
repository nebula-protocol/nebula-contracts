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