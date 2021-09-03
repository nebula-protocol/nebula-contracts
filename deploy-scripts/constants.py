from contract_helpers import Contract, ClusterContract
from bot_code.clusters.bullish_cross_recomp_deploy.src.recomp_deploy import BullishCrossRecomposer
from bot_code.clusters.next_doge_recomp_deploy.src.recomp_deploy import NextDogeRecomposer
from bot_code.clusters.momentum_recomp_deploy.src.recomp_deploy import MomentumTradingRecomposer
from bot_code.clusters.terra_ecosystem_recomp_deploy.src.recomp_deploy import TerraFullDilutedMcapRecomposer, get_terra_ecosystem_info
from bot_code.clusters.fab_mang_recomp_deploy.src.recomp_deploy import FABMANGRecomposer
from bot_code.clusters.future_of_france_recomp_deploy.src.recomp_deploy import FutureOfFranceRecomposer

terraswap_factory_teq = Contract("terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf")

DEPLOY_ENVIRONMENT_STATUS_W_GOV = {
  'airdrop': Contract("terra1r8ff3p8rxuverx3y52gd5a7r7vgw756vvthqcs"),
  'asset_prices': None,
  'asset_tokens': None,
  'cluster': None,
  'cluster_pair': None,
  'cluster_token': None,
  'code_ids': {'nebula_airdrop': '6906',
              'nebula_cluster': '6891',
              'nebula_cluster_factory': '6899',
              'nebula_collector': '6901',
              'nebula_community': '6905',
              'nebula_dummy_oracle': '6900',
              'nebula_gov': '6893',
              'nebula_incentives': '6903',
              'nebula_incentives_custody': '6902',
              'nebula_lp_staking': '6896',
              'nebula_penalty': '6895',
              'terraswap_factory': '6892',
              'terraswap_oracle': '6894',
              'terraswap_pair': '6897',
              'terraswap_router': '6904',
              'terraswap_token': '6898'},
  'collector': Contract("terra1jlkjgzzdej6784vpqfllqnayy65tfrswq9zef0"),
  'community': Contract("terra1rh5gvl0pz8t8u7mm58x4ywhnp6zrttp45wkmur"),
  'dummy_oracle': Contract("terra1ajjdnwvmhgc36p75apzrzkh2ekd8af3hqlzeka"),
  'factory': Contract("terra193w6tr2c2lyfhj2lnjekh9u94w97adh3d6ftu0"),
  'gov': Contract("terra14drm949cen0cny43zjfrdgyqffy7g4c08p0fzr"),
  'incentives': Contract("terra195e4pkkurjj0ul597yupare6z9kwyp6x092x63"),
  'incentives_custody': Contract("terra1qt36asnys8tgf4cqlw6ss2uzq0zv7nd07dcl9w"),
  'lp_token': None,
  'neb_pair': Contract("terra1as6rm64lfdet0w850t0ysvykmrlq8ll9lss35t"),
  'neb_token': Contract("terra13qhg3v5kpkmqm7tu4hetll0lvnjdlpth5a5w3t"),
  'require_gov': True,
  'staking': Contract("terra18fjkqd9hcyw3rlunfa69catnrr0stq4v80w2v4"),
  'terraswap_factory': Contract("terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf")
}

graphql_mir_data = {
  "data": {
    "assets": [
      {
        "symbol": "MIR",
        "name": "Mirror",
        "token": "terra15gwkyepfc6xgca5t5zefzwy42uts8l2m4g40k6"
      },
      {
        "symbol": "mABNB",
        "name": "Airbnb Inc.",
        "token": "terra1g4x2pzmkc9z3mseewxf758rllg08z3797xly0n"
      },
      {
        "symbol": "mETH",
        "name": "Ether",
        "token": "terra1dk3g53js3034x4v5c3vavhj2738une880yu6kx"
      },
      {
        "symbol": "mAAPL",
        "name": "Apple Inc.",
        "token": "terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz"
      },
      {
        "symbol": "mAMC",
        "name": "AMC Entertainment Holdings Inc.",
        "token": "terra1qelfthdanju7wavc5tq0k5r0rhsyzyyrsn09qy"
      },
      {
        "symbol": "mAMZN",
        "name": "Amazon.com, Inc.",
        "token": "terra165nd2qmrtszehcfrntlplzern7zl4ahtlhd5t2"
      },
      {
        "symbol": "mBABA",
        "name": "Alibaba Group Holding Limited",
        "token": "terra1w7zgkcyt7y4zpct9dw8mw362ywvdlydnum2awa"
      },
      {
        "symbol": "mBTC",
        "name": "Bitcoin",
        "token": "terra1rhhvx8nzfrx5fufkuft06q5marfkucdqwq5sjw"
      },
      {
        "symbol": "mFB",
        "name": "Facebook Inc.",
        "token": "terra1mqsjugsugfprn3cvgxsrr8akkvdxv2pzc74us7"
      },
      {
        "symbol": "mGME",
        "name": "GameStop Corp",
        "token": "terra1m6j6j9gw728n82k78s0j9kq8l5p6ne0xcc820p"
      },
      {
        "symbol": "mGOOGL",
        "name": "Alphabet Inc.",
        "token": "terra1h8arz2k547uvmpxctuwush3jzc8fun4s96qgwt"
      },
      {
        "symbol": "mGS",
        "name": "Goldman Sachs Group Inc.",
        "token": "terra137drsu8gce5thf6jr5mxlfghw36rpljt3zj73v"
      },
      {
        "symbol": "mMSFT",
        "name": "Microsoft Corporation",
        "token": "terra1227ppwxxj3jxz8cfgq00jgnxqcny7ryenvkwj6"
      },
      {
        "symbol": "mNFLX",
        "name": "Netflix, Inc.",
        "token": "terra1jsxngqasf2zynj5kyh0tgq9mj3zksa5gk35j4k"
      },
      {
        "symbol": "mQQQ",
        "name": "Invesco QQQ Trust",
        "token": "terra1csk6tc7pdmpr782w527hwhez6gfv632tyf72cp"
      },
      {
        "symbol": "mSLV",
        "name": "iShares Silver Trust",
        "token": "terra1kscs6uhrqwy6rx5kuw5lwpuqvm3t6j2d6uf2lp"
      },
      {
        "symbol": "mTSLA",
        "name": "Tesla, Inc.",
        "token": "terra14y5affaarufk3uscy2vr6pe6w6zqf2wpjzn5sh"
      },
      {
        "symbol": "mTWTR",
        "name": "Twitter, Inc.",
        "token": "terra1cc3enj9qgchlrj34cnzhwuclc4vl2z3jl7tkqg"
      },
      {
        "symbol": "mUSO",
        "name": "United States Oil Fund, LP",
        "token": "terra1lvmx8fsagy70tv0fhmfzdw9h6s3sy4prz38ugf"
      },
      {
        "symbol": "mSPY",
        "name": "SPDR S&P 500",
        "token": "terra1aa00lpfexyycedfg5k2p60l9djcmw0ue5l8fhc"
      },
      {
        "symbol": "mCOIN",
        "name": "Coinbase Global, Inc.",
        "token": "terra18wayjpyq28gd970qzgjfmsjj7dmgdk039duhph"
      },
      {
        "symbol": "mGLXY",
        "name": "Galaxy Digital Holdings Ltd",
        "token": "terra1l5lrxtwd98ylfy09fn866au6dp76gu8ywnudls"
      },
      {
        "symbol": "mIAU",
        "name": "iShares Gold Trust",
        "token": "terra10h7ry7apm55h4ez502dqdv9gr53juu85nkd4aq"
      },
      {
        "symbol": "mVIXY",
        "name": "ProShares VIX Short-Term Futures ETF",
        "token": "terra19cmt6vzvhnnnfsmccaaxzy2uaj06zjktu6yzjx"
      }
    ]
  }
}

SYM_TO_CONTRACT_TOKEN_TEQ = {
    'MIR': "terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd",
    'mAAPL': "terra1pwd9etdemugqdt92t5d3g98069z0axpz9plnsk",
    'mABNB': "terra1jm4j6k0e2dpug7z0glc87lwvyqh40z74f40n52",
    'mAMC': "terra1wa87zjty4y983yyt604hdnyr8rm9mwz7let8uz",
    'mAMZN': "terra18mjauk9ug8y29q678c2qlee6rkd9aunrpe9q97",
    'mBABA': "terra1uvzz9fchferxpg64pdshnrc49zkxjcj66uppq8",
    'mBTC': "terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p",
    'mCOIN': "terra16e3xu8ly6a622tjykfuwuv80czexece8rz0gs5",
    'mETH': "terra1rxyctpwzqvldalafvry787thslne6asjlwqjhn",
    'mFB': "terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63",
    'mGLXY': "terra17sm265sez3qle769ef4hscx540wem5hvxztpxg",
    'mGME': "terra19y6tdnps3dsd7qc230tk3jplwl9jm27mpcx9af",
    'mGOOGL': "terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524",
    'mGS': "terra199yfqa5092v2udw0k0h9rau9dzel0jkf5kk3km",
    'mIAU': "terra1n7pd3ssr9sqacwx5hekxsmdy86lwlm0fsdvnwe",
    'mMSFT': "terra18aztjeacdfc5s30ms0558cy8lygvam3s4v69jg",
    'mNFLX': "terra1smu8dc2xpa9rfj525n3a3ttgwnacnjgr59smu7",
    'mQQQ': "terra1r20nvsd08yujq29uukva8fek6g32p848kzlkfc",
    'mSLV': "terra1re6mcpu4hgzs5wc77gffsluqauanhpa8g7nmjc",
    'mSPY': "terra1j3l2ul7s8fkaadwdan67hejt7k5nylmxfkwg0w",
    'mTSLA': "terra1k44gg67rnc6av8sn0602876w8we5lu3jp30yec",
    'mTWTR': "terra1897xd8jqjkfpr5496ur8n896gd8fud3shq3t4q",
    'mUSO': "terra1c3nyehgvukzrt5k9lxzzw64d68el6cejyxjqde",
    'mVIXY': "terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9",
    'AAVE': 'terra1rw388r5ptypzzeyqr2swc44drju3zu2j5qlaw2',
    'ANC': 'terra16z5t7cr0ueg47tuqmwlp6ymgm2w43dyv4xnt4g',
    'COMP': 'terra1hmuuk7230na78mgp67kf4f0qenyw9xhfjzhaay',
    'CREAM': 'terra1dvx9np7ajmky66kz8r4dvze9e6gwsxwz5h6x4d',
    'CUMMIES': 'terra1kf9qa5f3uu7nq3flg2dva8c9d9lh8h5cyuextt',
    'DOGE': 'terra1wpa2978x6n9c6xdvfzk4uhkzvphmq5fhdnvrym',
    'ERCTWENTY': 'terra1p0rp8he7jfnevha3k5anhd0als7azjmfhxrvjv',
    'MEME': 'terra1u08z2c9r3s3avrn9l0r3m30xhcmssunvv5d0rx',
    'MKR': 'terra13rkv7zdg4huwe0z9c0k8t7gc3hxhy58c3zghec'
 }

CONTRACT_TOKEN_TO_SYM_TEQ = {
    'terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9': 'mVIXY',
    'terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p': 'mBTC',
    'terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524': 'mGOOGL',
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
    'terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63': 'mFB',
    'terra13rkv7zdg4huwe0z9c0k8t7gc3hxhy58c3zghec': 'MKR',
    'terra16z5t7cr0ueg47tuqmwlp6ymgm2w43dyv4xnt4g': 'ANC',
    'terra1dvx9np7ajmky66kz8r4dvze9e6gwsxwz5h6x4d': 'CREAM',
    'terra1hmuuk7230na78mgp67kf4f0qenyw9xhfjzhaay': 'COMP',
    'terra1kf9qa5f3uu7nq3flg2dva8c9d9lh8h5cyuextt': 'CUMMIES',
    'terra1p0rp8he7jfnevha3k5anhd0als7azjmfhxrvjv': 'ERC20',
    'terra1rw388r5ptypzzeyqr2swc44drju3zu2j5qlaw2': 'AAVE',
    'terra1u08z2c9r3s3avrn9l0r3m30xhcmssunvv5d0rx': 'MEME',
    'terra1wpa2978x6n9c6xdvfzk4uhkzvphmq5fhdnvrym': 'DOGE'
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
    'MEME': 'degenerator'
}

# Helper for deploying
CT_SYM_TO_NAME = { 
    'TER': 'Terra Ecosystem',
    'FABMANG': 'FAB MANG',
    'MOMENTUM': 'Top 5 30-Day Momentum',
    'BULL': 'Bullish Cross',
    'FOF': 'The Future of France',
    'NOGE': 'The Next Doge',
}

# Helper for deploying
CT_SYM_TO_RECOMP_ORACLE = { 
    'TER': 'terra14ew659y4fn4dytu832k9f6l2u94668uclrywfg',
    'FABMANG': 'terra1k5ymxx67tl6cd3dk4kxwt7mwl03dnuqggm8vsv',
    'MOMENTUM': 'terra1m0z2ul2kzz2ua2fttstn0wkm2fp500pm9am396',
    'BULL': 'terra1e3u7msymmkxu8u68rdvg0nqmq7zaafttcq64ky',
    'FOF': 'terra17qfm7gmtcup5tjq99rhm7a685x6vffg3v8u3wk',
    'NOGE': 'terra14s0lccdf88wq7t0hxkzwwnglskh588dy0nxzkc',
}

# Helper for deploying
CT_SYM_TO_RECOMPOSER = { 
    'TER': TerraFullDilutedMcapRecomposer,
    'FABMANG': FABMANGRecomposer,
    'MOMENTUM': MomentumTradingRecomposer,
    'BULL': BullishCrossRecomposer,
    'FOF': FutureOfFranceRecomposer,
    'NOGE': NextDogeRecomposer,
}