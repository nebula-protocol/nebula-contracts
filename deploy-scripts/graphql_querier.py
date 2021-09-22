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

# Dummy contract on tequila to symbol
CONTRACT_TOKEN_TO_SYM_BOMBAY = {
  'terra10af2zy62wanc6cs3n66cplmpepvf6qnetuydz2': 'COMP',
  'terra14vxe68djqpmzspvkaj9fjxc8fu6qmt34wmm6xc': 'ENJ',
  'terra1a7g946jyjhn8h7gscda7sd68kn9k4whkxq0ddn': 'CREAM',
  'terra1exw6sae4wyq8rt56hxdggzmgmqsuukr26u4aj8': 'AAVE',
  'terra1lc6czeag9zaaqk04y5ynfkxu723n7t56kg2a9r': 'MANA',
  'terra1lflvesvarcfu53gd9cgkv3juyrz79cnk7yw6am': 'MKR',
  'terra1mst8t7guwkku9rqhre4lxtkfkz3epr45wt8h0m': 'ANC',
  'terra1q3smy9j5qjplyas4l3tgyj72qtq9fvysff4msa': 'SAND',
  'terra1t89u7cfrp9r4a8msmxz4z3esn5g5z8ga2qsec6': 'AUDIO',
  'terra1w07h8u34an2jcfsegjc80edunngf3ey6xdz456': 'AXS',
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
    'AAVE': 'terra1exw6sae4wyq8rt56hxdggzmgmqsuukr26u4aj8',
    'ANC': 'terra1mst8t7guwkku9rqhre4lxtkfkz3epr45wt8h0m',
    'AUDIO': 'terra1t89u7cfrp9r4a8msmxz4z3esn5g5z8ga2qsec6',
    'AXS': 'terra1w07h8u34an2jcfsegjc80edunngf3ey6xdz456',
    'COMP': 'terra10af2zy62wanc6cs3n66cplmpepvf6qnetuydz2',
    'CREAM': 'terra1a7g946jyjhn8h7gscda7sd68kn9k4whkxq0ddn',
    'ENJ': 'terra14vxe68djqpmzspvkaj9fjxc8fu6qmt34wmm6xc',
    'MANA': 'terra1lc6czeag9zaaqk04y5ynfkxu723n7t56kg2a9r',
    'MKR': 'terra1lflvesvarcfu53gd9cgkv3juyrz79cnk7yw6am',
    'SAND': 'terra1q3smy9j5qjplyas4l3tgyj72qtq9fvysff4msa',
    'MIR': 'terra159nvmamkrj0hw5e0e0lp4vzh6py0ev765jgl58',
    'mAAPL': 'terra1qkk30fyqn27fz0a0h7alx6h73pjhur0afxlamy',
    'mABNB': 'terra10x0h5r0t9hdwamdxehapjnj67p4f8nx38pxuzx',
    'mAMC': 'terra1kvjetgk5arnsyn4t4cer8ppttdlymcn35awdc7',
    'mAMZN': 'terra1gua38jnfldhrqw6xgshwe4phkdyuasfnv5jfyu',
    'mBABA': 'terra1fczn32j5zt0p9u9eytxa7cdzvhu7yll06lzvl3',
    'mBTC': 'terra14js9dgr87dxepx2gczkudxj69xudf2npnw87f9',
    'mCOIN': 'terra1ml4dh06egs4ezjhq5r50ku3zc8086yfsvtreyl',
    'mETH': 'terra1p8qzs0glqkfx6e08alr5c66vnlscl09df2wmwa',
    'mFB': 'terra1ucsa089wnu7u6qe05ujp4vzvf73u9aq3u89ytn',
    'mGLXY': 'terra1et7mmctaffrg0sczfaxkqwksd43wt5fvjhyffd',
    'mGME': 'terra1ud984ssduc53q6z90raydwe4akch98q0ksr5ry',
    'mGOOGL': 'terra1ym8kp806plgum787fxpukj6z8tg90eslklppfq',
    'mGS': 'terra1lj8x2s06vmfherel08qptvv22wqld0z3ytmzcf',
    'mIAU': 'terra1xx5ndkhe477sa267fc6mryq7jekk6aczep6mqh',
    'mMSFT': 'terra1kqqqhtqsu9h4c93rlmteg2zuhc0z53ewlwt8vq',
    'mNFLX': 'terra1r27h7zpchq40r54x64568yep3x8j93lr5u2g24',
    'mQQQ': 'terra1wxjq2lsxvhq90z0muv4nkcddjt23t89vh4s4d6',
    'mSLV': 'terra1gytrpc5972ed3gthmupmc6wxyayx4wtvmzq8cy',
    'mTSLA': 'terra1lrtvldvfkxx47releuk266numcg2k29y7t8t2n',
    'mTWTR': 'terra1a4jtyzta9zr3df8w2f5d8zr44ws0dm58cznsca',
    'mUSO': 'terra1tpsls0lzyh2fkznhjyes56upgk5g4z0sw3hgdn',
    'mVIXY': 'terra1yvplcammukw0d5583jw4payn0veqtgfumqvjk0'
 }

SYM_TO_CONTRACT_TOKEN_BOMBAY_11 = {
  'MIR': 'terra1vx0esu27cfkswurt646x3mhfh4wvlwpf4g5t6l',
  'mAAPL': 'terra1j7twctynt5570e368e4vz5yk48f75vnmkkadw5',
  'mABNB': 'terra19k40atlcyztyqe6k0d6skesevs2yc2k02dtkdx',
  'mAMC': 'terra17dlj5a2zj0y2qmv077pt35d0ftw8k6aran39g4',
  'mAMZN': 'terra1er94ptw07ruqgprd82482nwxcvk5ms9x2egvpj',
  'mBABA': 'terra1vmcw3z6z02tgw49lwvyshspceqd52la7u2n735',
  'mBTC': 'terra1r3tkv6en0xzfa9c0e5g3rkfwashhjl7rglwkh4',
  'mCOIN': 'terra1swyrkzxssfypslxk75e66rr48w7gc5eacq4vme',
  'mETH': 'terra1y3dzlanz3sr2n08qgc7dt2kw5yx5c4364kq6jc',
  'mFB': 'terra1nxm5c4e2609sjfwjd6uskywdkh866phpth5ctx',
  'mGLXY': 'terra1r6k0qadau4egeftxrsl0spvsvw2tneelamymdg',
  'mGME': 'terra1mf3qumw3w8uyx7tz7h6jq2y5g5v6urlvkmw5yg',
  'mGOOGL': 'terra1396cs9x0h0nnk0tpuwh2et6qraqh9gkl6h29wu',
  'mGS': 'terra18ftfpy79us04teq6nc35e3vd6f253unamqalnj',
  'mIAU': 'terra1m4t75uevjdr8mrz8evxs7wcvrlxl5p6d3lmhev',
  'mMSFT': 'terra1lkht9dsqzmcdxkx8gryuyevmp9ca9rk3xup9sf',
  'mNFLX': 'terra1rlwmalf5yjkdl7dc69rta9u09tnkvdfe6qw2px',
  'mQQQ': 'terra1d0ngfh0n65nsm5wnktfucenr549vdsguk07ug8',
  'mSLV': 'terra14322huutrc3pmdazpg7h6v7lxay2dz42q60xcl',
  'mTSLA': 'terra19yu3y6lzvashmfc9hpqz8pund2w3k6fqdwfp7p',
  'mTWTR': 'terra126a48h56774pse5argad5myx62wfhz32me9lcr',
  'mUSO': 'terra1g8rh8etjcjz0jg92vnalp69u07gmj9me4y9dzl',
  'mVIXY': 'terra169fj4nzrryaj3l9m7wyxkgee9j7jqf2yn9uzkz',
  'AAVE': 'terra1j9dmu4k9jm8e52yr2rz20rlhl9kzg9xmd9qszu',
  'ANC': 'terra1jffn63c4tfzg66qdcaznhqaphmgvprp7muauq6',
  'AUDIO': 'terra16gclfyjnqcdjplx2lxp6f6a9agg7czd5ewmhm5',
  'AXS': 'terra1kjqdwxdgdw8pphtdvu43kwtl0d8q93j25u6rl2',
  'COMP': 'terra1vacxu53744tze80e0yjpsyp4rm27s5dr8fvagv',
  'CREAM': 'terra1xdhrww6v3dxy20ss8gqdk2mwwfk0gqq9ggzf7a',
  'ENJ': 'terra10tey9kxcutm4hzwqnf87mg2sygqx4dvfc50795',
  'MANA': 'terra1awdn73w3rllhymnq7getht66tr4cvhgm43zx33',
  'MKR': 'terra1gm05r4aqmfywp8x2ruphacc4xtnlpuk57m8v2y',
  'SAND': 'terra1uctdrs0f2g2usgyfhujaehj8e9pfajsy4emk4t',
}

CONTRACT_TOKEN_TO_SYM_BOMBAY_11 = {
  'terra126a48h56774pse5argad5myx62wfhz32me9lcr': 'mTWTR',
  'terra1396cs9x0h0nnk0tpuwh2et6qraqh9gkl6h29wu': 'mGOOGL',
  'terra14322huutrc3pmdazpg7h6v7lxay2dz42q60xcl': 'mSLV',
  'terra169fj4nzrryaj3l9m7wyxkgee9j7jqf2yn9uzkz': 'mVIXY',
  'terra17dlj5a2zj0y2qmv077pt35d0ftw8k6aran39g4': 'mAMC',
  'terra18ftfpy79us04teq6nc35e3vd6f253unamqalnj': 'mGS',
  'terra19k40atlcyztyqe6k0d6skesevs2yc2k02dtkdx': 'mABNB',
  'terra19yu3y6lzvashmfc9hpqz8pund2w3k6fqdwfp7p': 'mTSLA',
  'terra1d0ngfh0n65nsm5wnktfucenr549vdsguk07ug8': 'mQQQ',
  'terra1er94ptw07ruqgprd82482nwxcvk5ms9x2egvpj': 'mAMZN',
  'terra1g8rh8etjcjz0jg92vnalp69u07gmj9me4y9dzl': 'mUSO',
  'terra1j7twctynt5570e368e4vz5yk48f75vnmkkadw5': 'mAAPL',
  'terra1lkht9dsqzmcdxkx8gryuyevmp9ca9rk3xup9sf': 'mMSFT',
  'terra1m4t75uevjdr8mrz8evxs7wcvrlxl5p6d3lmhev': 'mIAU',
  'terra1mf3qumw3w8uyx7tz7h6jq2y5g5v6urlvkmw5yg': 'mGME',
  'terra1nxm5c4e2609sjfwjd6uskywdkh866phpth5ctx': 'mFB',
  'terra1r3tkv6en0xzfa9c0e5g3rkfwashhjl7rglwkh4': 'mBTC',
  'terra1r6k0qadau4egeftxrsl0spvsvw2tneelamymdg': 'mGLXY',
  'terra1rlwmalf5yjkdl7dc69rta9u09tnkvdfe6qw2px': 'mNFLX',
  'terra1swyrkzxssfypslxk75e66rr48w7gc5eacq4vme': 'mCOIN',
  'terra1vmcw3z6z02tgw49lwvyshspceqd52la7u2n735': 'mBABA',
  'terra1vx0esu27cfkswurt646x3mhfh4wvlwpf4g5t6l': 'MIR',
  'terra1y3dzlanz3sr2n08qgc7dt2kw5yx5c4364kq6jc': 'mETH',
  'terra10tey9kxcutm4hzwqnf87mg2sygqx4dvfc50795': 'ENJ',
  'terra16gclfyjnqcdjplx2lxp6f6a9agg7czd5ewmhm5': 'AUDIO',
  'terra1awdn73w3rllhymnq7getht66tr4cvhgm43zx33': 'MANA',
  'terra1gm05r4aqmfywp8x2ruphacc4xtnlpuk57m8v2y': 'MKR',
  'terra1j9dmu4k9jm8e52yr2rz20rlhl9kzg9xmd9qszu': 'AAVE',
  'terra1jffn63c4tfzg66qdcaznhqaphmgvprp7muauq6': 'ANC',
  'terra1kjqdwxdgdw8pphtdvu43kwtl0d8q93j25u6rl2': 'AXS',
  'terra1uctdrs0f2g2usgyfhujaehj8e9pfajsy4emk4t': 'SAND',
  'terra1vacxu53744tze80e0yjpsyp4rm27s5dr8fvagv': 'COMP',
  'terra1xdhrww6v3dxy20ss8gqdk2mwwfk0gqq9ggzf7a': 'CREAM',
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