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

    # # TODO: CHANGE FROM TESTNET
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

CONTRACT_TOKEN_TO_SYM_TEQ = {
    'terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9': 'mVIXY',
    'terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p': 'mBTC',
    'terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524': 'mGOOGL',
    'terra15tecrcm27fenchxaqde9f8ws8krfgjnqf2hhcv': 'ANC',
    'terra16e3xu8ly6a622tjykfuwuv80czexece8rz0gs5': 'mCOIN',
    'terra17sm265sez3qle769ef4hscx540wem5hvxztpxg': 'mGLXY',
    'terra1897xd8jqjkfpr5496ur8n896gd8fud3shq3t4q': 'mTWTR',
    'terra18aztjeacdfc5s30ms0558cy8lygvam3s4v69jg': 'mMSFT',
    'terra18mjauk9ug8y29q678c2qlee6rkd9aunrpe9q97': 'mAMZN',
    'terra199yfqa5092v2udw0k0h9rau9dzel0jkf5kk3km': 'mGS',
    'terra19y6tdnps3dsd7qc230tk3jplwl9jm27mpcx9af': 'mGME',
    'terra1c3nyehgvukzrt5k9lxzzw64d68el6cejyxjqde': 'mUSO',
    'terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd': 'MIR',
    'terra1j3l2ul7s8fkaadwdan67hejt7k5nylmxfkwg0w': 'mSPY',
    'terra1jm4j6k0e2dpug7z0glc87lwvyqh40z74f40n52': 'mABNB',
    'terra1k44gg67rnc6av8sn0602876w8we5lu3jp30yec': 'mTSLA',
    'terra1n7pd3ssr9sqacwx5hekxsmdy86lwlm0fsdvnwe': 'mIAU',
    'terra1pwd9etdemugqdt92t5d3g98069z0axpz9plnsk': 'mAAPL',
    'terra1r20nvsd08yujq29uukva8fek6g32p848kzlkfc': 'mQQQ',
    'terra1re6mcpu4hgzs5wc77gffsluqauanhpa8g7nmjc': 'mSLV',
    'terra1rxyctpwzqvldalafvry787thslne6asjlwqjhn': 'mETH',
    'terra1smu8dc2xpa9rfj525n3a3ttgwnacnjgr59smu7': 'mNFLX',
    'terra1uvzz9fchferxpg64pdshnrc49zkxjcj66uppq8': 'mBABA',
    'terra1wa87zjty4y983yyt604hdnyr8rm9mwz7let8uz': 'mAMC',
    'terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63': 'mFB'
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
    return list(CONTRACT_TOKEN_TO_SYM_TEQ.keys())