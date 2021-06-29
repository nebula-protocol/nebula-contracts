import requests
import json
import pandas as pd

def mirror_history_query(tick, from_stamp, to_stamp):
    query = """
    query {{
        asset(token: "terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz") {{
            prices {{
                history(interval: {0}, from: {1}, to: {2}) {{
                    timestamp
                    price
                }}
            }},
            symbol,
            name
        }}
    }}""".format(tick, from_stamp, to_stamp)

    url = 'https://graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    if r.status_code != 200:
        print("Error!")
        return None
    asset = json.loads(r.text)['data']['asset']

    prices = asset['prices']['history']
    symbol = asset['symbol']
    latest_timestamp = max([p['timestamp'] for p in prices])
    closes = [p['price'] for p in prices]

    return symbol, latest_timestamp, closes