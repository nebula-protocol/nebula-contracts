import requests
import json
import pandas as pd

async def mirror_history_query(address, tick, from_stamp, to_stamp):
    query = """
    query {{
        asset(token: {0}) {{
            prices {{
                oracleHistory(interval: {1}, from: {2}, to: {3}) {{
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

    # From testnet
    # url = 'https://tequila-graph.mirror.finance/graphql'
    r = requests.post(url, json={'query': query})
    try:
        r.raise_for_status()
        asset = json.loads(r.text)['data']['asset']

        prices = asset['prices']['oracleHistory']
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

# Symbol to mAsset address on Columbus
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

SYM_TO_COINGECKO_ID = { 
    'ANC': 'anchor-protocol',
    'uluna': 'terra-luna',
    'uusd': 'terrausd',
    'MIR': 'mirror-protocol', # can use graphql for this one
    'UST': 'terrausd',
    'AAVE': 'aave',
    'COMP': 'compound-governance-token',
    'MKR': 'maker',
    'CREAM': 'cream-2',
    'DOGE': 'dogecoin',
    'ERCTWENTY': 'erc20',
    'CUMMIES': 'cumrocket',
    'MEME': 'degenerator',
    'AXS': 'axie-infinity',
    'SAND': 'the-sandbox',
    'MANA': 'decentraland',
    'ENJ': 'enjincoin',
    'AUDIO': 'audius'
}

SYM_TO_CONTRACT_TOKEN_BOMBAY = {
    'MIR': 'terra1985qhth953j2tyq8mlszghnxs57veth38866xu',
    'mAAPL': 'terra1648p53nc9eaa8h7md6p09npuwc97hrx99xv7cz',
    'mABNB': 'terra1w6g0hk5fp3klp0lp4cp296vsjyyg2rlt3xwvtq',
    'mAMC': 'terra1jlxsvh93gehv6e3z97nluz8h87h3thpwyup9v9',
    'mAMZN': 'terra1v5mhc7v3ar0qe2zkhwzgw4hrash7wl77tpf98h',
    'mBABA': 'terra10te74l9e0g7a60g5lluk8fhq5znplulz0xjexq',
    'mBTC': 'terra1f6ezwstauw3seqah6ejpvvakv6humm939whrts',
    'mCOIN': 'terra1dr2lsmsclm6zq5cmlrd755vweq95dm2v5jhwap',
    'mETH': 'terra1tluve57f8cf4zjhwkjn7k0ed2zyaw44qxdcfsl',
    'mFB': 'terra1n72had3dfk2qvp77xha8wagk38um0ts6y58svw',
    'mGLXY': 'terra1ferkwr5tx4anwth0dey2lxjj74fxfgsr60555c',
    'mGME': 'terra1cjj0f09rhprhr9qrs4pxcfmhmy2rmuvd680ta4',
    'mGOOGL': 'terra1vrpc6ufn6rv7suyyppl6tan7jf74a2578z4yur',
    'mGS': 'terra1z4ezfd0uxq7qj25m0tpk68wr59hv8ga2f06xd0',
    'mIAU': 'terra1ellhdys6lxedwk4ts6th8ecea7c6c0298sv6j9',
    'mMSFT': 'terra18a4jmmpgwcle73jj2jhk7kfnwh4uydq78ghctj',
    'mNFLX': 'terra1y82js5dfyh2wnahhx6qjfzq0vn046ved9k3nx2',
    'mQQQ': 'terra1cc7fcskxew7j9yy708grkyhlryns5qrgdaadum',
    'mSLV': 'terra12k75zuh2k327llm64r2vyw2hpt9tx2tnepy05m',
    'mTSLA': 'terra15ud868qps4nap4fl335tunqmqxauc5ujmwy62t',
    'mTWTR': 'terra16m9lcszq3qcc4etcq58v2lpx37vxjnzm2luwf6',
    'mVIXY': 'terra1sedca388l3q40jnkc502g5t738944xjh5hfvsc',
    'AAVE': 'terra18cm73232f69reflhj4ch0frzjnyx4vrn5tv3rl',
    'ANC': 'terra1sjras9rmeu7xpzdscxlmlhnq2erz3kwwvu5gzx',
    'AUDIO': 'terra1smm572lhpu2zwn0k656ns6ecxlk6scljse88h5',
    'AXS': 'terra1wdsw90vepru5v7w7qnwk3npqa75q469am9n24g',
    'COMP': 'terra1r0hluxa6mn7hyrg9j78l0l0f59dnhhsfgs0qze',
    'CREAM': 'terra17j3h8mysp9pkjyyueqnutwk4rzuhtuldfsf0q2',
    'ENJ': 'terra1tytzk7mhyqja2xp9yluwjhrc9htkx8tqslc0fg',
    'MANA': 'terra1qrmp67k6tr42lzk6cv0rhhrh2pyeaqekcqax9x',
    'MKR': 'terra12j2vx0hufdg5y8zuj3vw2f6u3ukrgs3gnqrzjg',
    'SAND': 'terra185apwn7el77v7cupxwlc5vagemjvwaqp8y8qje'
 }

CONTRACT_TOKEN_TO_SYM_BOMBAY = {
    'terra10te74l9e0g7a60g5lluk8fhq5znplulz0xjexq': 'mBABA',
    'terra12k75zuh2k327llm64r2vyw2hpt9tx2tnepy05m': 'mSLV',
    'terra15ud868qps4nap4fl335tunqmqxauc5ujmwy62t': 'mTSLA',
    'terra1648p53nc9eaa8h7md6p09npuwc97hrx99xv7cz': 'mAAPL',
    'terra16m9lcszq3qcc4etcq58v2lpx37vxjnzm2luwf6': 'mTWTR',
    'terra18a4jmmpgwcle73jj2jhk7kfnwh4uydq78ghctj': 'mMSFT',
    'terra1985qhth953j2tyq8mlszghnxs57veth38866xu': 'MIR',
    'terra1cc7fcskxew7j9yy708grkyhlryns5qrgdaadum': 'mQQQ',
    'terra1cjj0f09rhprhr9qrs4pxcfmhmy2rmuvd680ta4': 'mGME',
    'terra1dr2lsmsclm6zq5cmlrd755vweq95dm2v5jhwap': 'mCOIN',
    'terra1ellhdys6lxedwk4ts6th8ecea7c6c0298sv6j9': 'mIAU',
    'terra1f6ezwstauw3seqah6ejpvvakv6humm939whrts': 'mBTC',
    'terra1ferkwr5tx4anwth0dey2lxjj74fxfgsr60555c': 'mGLXY',
    'terra1jlxsvh93gehv6e3z97nluz8h87h3thpwyup9v9': 'mAMC',
    'terra1n72had3dfk2qvp77xha8wagk38um0ts6y58svw': 'mFB',
    'terra1sedca388l3q40jnkc502g5t738944xjh5hfvsc': 'mVIXY',
    'terra1tluve57f8cf4zjhwkjn7k0ed2zyaw44qxdcfsl': 'mETH',
    'terra1v5mhc7v3ar0qe2zkhwzgw4hrash7wl77tpf98h': 'mAMZN',
    'terra1vrpc6ufn6rv7suyyppl6tan7jf74a2578z4yur': 'mGOOGL',
    'terra1w6g0hk5fp3klp0lp4cp296vsjyyg2rlt3xwvtq': 'mABNB',
    'terra1y82js5dfyh2wnahhx6qjfzq0vn046ved9k3nx2': 'mNFLX',
    'terra1z4ezfd0uxq7qj25m0tpk68wr59hv8ga2f06xd0': 'mGS',
    'terra12j2vx0hufdg5y8zuj3vw2f6u3ukrgs3gnqrzjg': 'MKR',
    'terra17j3h8mysp9pkjyyueqnutwk4rzuhtuldfsf0q2': 'CREAM',
    'terra185apwn7el77v7cupxwlc5vagemjvwaqp8y8qje': 'SAND',
    'terra18cm73232f69reflhj4ch0frzjnyx4vrn5tv3rl': 'AAVE',
    'terra1qrmp67k6tr42lzk6cv0rhhrh2pyeaqekcqax9x': 'MANA',
    'terra1r0hluxa6mn7hyrg9j78l0l0f59dnhhsfgs0qze': 'COMP',
    'terra1sjras9rmeu7xpzdscxlmlhnq2erz3kwwvu5gzx': 'ANC',
    'terra1smm572lhpu2zwn0k656ns6ecxlk6scljse88h5': 'AUDIO',
    'terra1tytzk7mhyqja2xp9yluwjhrc9htkx8tqslc0fg': 'ENJ',
    'terra1wdsw90vepru5v7w7qnwk3npqa75q469am9n24g': 'AXS'
}

async def mirror_history_query_test(address, tick, from_stamp, to_stamp):
    """
    Takes in test address linked to a symbol and return price history of symbol on Col-4
    """

    try:
        sym = CONTRACT_TOKEN_TO_SYM_BOMBAY[address]
        col_address = SYM_TO_MASSET_COL[sym]
    except:
        raise NameError

    query = """
    query {{
        asset(token: {0}) {{
            prices {{
                oracleHistory(interval: {1}, from: {2}, to: {3}) {{
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

        prices = asset['prices']['oracleHistory']
        symbol = asset['symbol']
        mcap = asset['statistic']['marketCap']
        latest_timestamp = max([p['timestamp'] for p in prices])
        closes = [p['price'] for p in prices]
        return symbol, latest_timestamp, closes, mcap
    except:
        return None, None, None, None
    

async def get_all_mirror_assets_test():
   return [k for k, v in CONTRACT_TOKEN_TO_SYM_BOMBAY.items() if (v[0] == 'm' or v == 'MIR')]