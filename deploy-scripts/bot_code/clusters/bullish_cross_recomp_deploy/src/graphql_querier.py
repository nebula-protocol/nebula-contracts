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

    url = 'https://graph.mirror.finance/graphql'

    # For testnet
    # url = 'https://tequila-graph.mirror.finance/graphql'
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

CONTRACT_TOKEN_TO_SYM_BOMBAY = {
  'terra10x0h5r0t9hdwamdxehapjnj67p4f8nx38pxuzx': 'mABNB',
  'terra14js9dgr87dxepx2gczkudxj69xudf2npnw87f9': 'mBTC',
  'terra159nvmamkrj0hw5e0e0lp4vzh6py0ev765jgl58': 'MIR',
  'terra1a4jtyzta9zr3df8w2f5d8zr44ws0dm58cznsca': 'mTWTR',
  'terra1et7mmctaffrg0sczfaxkqwksd43wt5fvjhyffd': 'mGLXY',
  'terra1fczn32j5zt0p9u9eytxa7cdzvhu7yll06lzvl3': 'mBABA',
  'terra1gua38jnfldhrqw6xgshwe4phkdyuasfnv5jfyu': 'mAMZN',
  'terra1gytrpc5972ed3gthmupmc6wxyayx4wtvmzq8cy': 'mSLV',
  'terra1kqqqhtqsu9h4c93rlmteg2zuhc0z53ewlwt8vq': 'mMSFT',
  'terra1kvjetgk5arnsyn4t4cer8ppttdlymcn35awdc7': 'mAMC',
  'terra1lj8x2s06vmfherel08qptvv22wqld0z3ytmzcf': 'mGS',
  'terra1lrtvldvfkxx47releuk266numcg2k29y7t8t2n': 'mTSLA',
  'terra1ml4dh06egs4ezjhq5r50ku3zc8086yfsvtreyl': 'mCOIN',
  'terra1p8qzs0glqkfx6e08alr5c66vnlscl09df2wmwa': 'mETH',
  'terra1qkk30fyqn27fz0a0h7alx6h73pjhur0afxlamy': 'mAAPL',
  'terra1r27h7zpchq40r54x64568yep3x8j93lr5u2g24': 'mNFLX',
  'terra1tpsls0lzyh2fkznhjyes56upgk5g4z0sw3hgdn': 'mUSO',
  'terra1ucsa089wnu7u6qe05ujp4vzvf73u9aq3u89ytn': 'mFB',
  'terra1ud984ssduc53q6z90raydwe4akch98q0ksr5ry': 'mGME',
  'terra1wxjq2lsxvhq90z0muv4nkcddjt23t89vh4s4d6': 'mQQQ',
  'terra1xx5ndkhe477sa267fc6mryq7jekk6aczep6mqh': 'mIAU',
  'terra1ym8kp806plgum787fxpukj6z8tg90eslklppfq': 'mGOOGL',
  'terra1yvplcammukw0d5583jw4payn0veqtgfumqvjk0': 'mVIXY',
}

SYM_TO_MASSET_COL = {
    'MIR': 'terra15gwkyepfc6xgca5t5zefzwy42uts8l2m4g40k6',
    'mAAPL': 'terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz',
    'mABNB': 'terra1g4x2pzmkc9z3mseewxf758rllg08z3797xly0n',
    'mAMC': 'terra1qelfthdanju7wavc5tq0k5r0rhsyzyyrsn09qy',
    'mAMZN': 'terra165nd2qmrtszehcfrntlplzern7zl4ahtlhd5t2',
    'mBABA': 'terra1w7zgkcyt7y4zpct9dw8mw362ywvdlydnum2awa',
    'mBTC': 'terra1rhhvx8nzfrx5fufkuft06q5marfkucdqwq5sjw',
    'mCOIN': 'terra18wayjpyq28gd970qzgjfmsjj7dmgdk039duhph',
    'mETH': 'terra1dk3g53js3034x4v5c3vavhj2738une880yu6kx',
    'mFB': 'terra1mqsjugsugfprn3cvgxsrr8akkvdxv2pzc74us7',
    'mGLXY': 'terra1l5lrxtwd98ylfy09fn866au6dp76gu8ywnudls',
    'mGME': 'terra1m6j6j9gw728n82k78s0j9kq8l5p6ne0xcc820p',
    'mGOOGL': 'terra1h8arz2k547uvmpxctuwush3jzc8fun4s96qgwt',
    'mGS': 'terra137drsu8gce5thf6jr5mxlfghw36rpljt3zj73v',
    'mIAU': 'terra10h7ry7apm55h4ez502dqdv9gr53juu85nkd4aq',
    'mMSFT': 'terra1227ppwxxj3jxz8cfgq00jgnxqcny7ryenvkwj6',
    'mNFLX': 'terra1jsxngqasf2zynj5kyh0tgq9mj3zksa5gk35j4k',
    'mQQQ': 'terra1csk6tc7pdmpr782w527hwhez6gfv632tyf72cp',
    'mSLV': 'terra1kscs6uhrqwy6rx5kuw5lwpuqvm3t6j2d6uf2lp',
    'mSPY': 'terra1aa00lpfexyycedfg5k2p60l9djcmw0ue5l8fhc',
    'mTSLA': 'terra14y5affaarufk3uscy2vr6pe6w6zqf2wpjzn5sh',
    'mTWTR': 'terra1cc3enj9qgchlrj34cnzhwuclc4vl2z3jl7tkqg',
    'mUSO': 'terra1lvmx8fsagy70tv0fhmfzdw9h6s3sy4prz38ugf',
    'mVIXY': 'terra19cmt6vzvhnnnfsmccaaxzy2uaj06zjktu6yzjx'
}

async def mirror_history_query_test(address, tick, from_stamp, to_stamp):
    """
    Takes in test address linked to a symbol and return price history of symbol on Col-4
    """

    try:
        sym = CONTRACT_TOKEN_TO_SYM_TEQ[address]
        col_address = SYM_TO_MASSET_COL[sym]
    except:
        raise NameError

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
    }}""".format('\"' + col_address + '\"', str(tick), str(from_stamp), str(to_stamp))

    url = 'https://graph.mirror.finance/graphql'

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
    

async def get_all_mirror_assets_test():
   return [k for k, v in CONTRACT_TOKEN_TO_SYM_TEQ.items() if (v[0] == 'm' or v == 'MIR')]