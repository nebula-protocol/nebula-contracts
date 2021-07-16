from contract_helpers import Contract, ClusterContract

DEPLOY_ENVIRONMENT_STATUS_W_GOV = {
  'airdrop': Contract("terra13ltrqg45wn2tzcesu6exvlyuhvv7tsfx50p22g"),
  'asset_tokens': None,
  'cluster': None,
  'cluster_pair': None,
  'cluster_token': None,
  'code_ids': {'nebula_airdrop': '6374',
              'nebula_cluster': '6359',
              'nebula_cluster_factory': '6367',
              'nebula_collector': '6369',
              'nebula_community': '6373',
              'nebula_dummy_oracle': '6368',
              'nebula_gov': '6361',
              'nebula_incentives': '6371',
              'nebula_incentives_custody': '6370',
              'nebula_lp_staking': '6364',
              'nebula_penalty': '6363',
              'terraswap_factory': '6360',
              'terraswap_oracle': '6362',
              'terraswap_pair': '6365',
              'terraswap_router': '6372',
              'terraswap_token': '6366'},
  'collector': Contract("terra1dsl76gx7j476nlfyurx6hac97ptay0srpu5u24"),
  'community': Contract("terra1kvgehdwlfj96h46tgwjahvacla90gj29h004nf"),
  'dummy_oracle': None,
  'factory': Contract("terra1hvlzmm4ggg938nka46a77e2fyyspg8slrcdntc"),
  'gov': Contract("terra1gd4qlxrz7em2s77hjxqpyvyq5fkg3e6u34scqr"),
  'incentives': Contract("terra1afqqeqq2jw6hs6sr46tj8u9q7vd0uz6hmtlxck"),
  'incentives_custody': Contract("terra19pjtjcdng5el694erm2mrk833r3xc9ady4rflr"),
  'lp_token': None,
  'neb_pair': Contract("terra1pxh7nmp5c3hdp4fnz4wdmar6mlx4f48lma75qs"),
  'neb_token': Contract("terra1jnqsg0nmn7efavrudreasghq2x4ja5d8wpqjzw"),
  'require_gov': True,
  'staking': Contract("terra1av7s7h7xxzzc3tl8up63flr76yz8lxcsqq4hma"),
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
    'uust': 'terrausd',
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