from contract_helpers import Contract, ClusterContract

DEPLOY_ENVIRONMENT_STATUS_W_GOV = {
  'airdrop': Contract("terra1ysuwgm6p6p5pe5y6yqva3jeaur9un5j9maw4v3"),
  'asset_tokens': None,
  'cluster': None,
  'cluster_pair': None,
  'cluster_token': None,
  'code_ids': {'basket_incentives': '6006',
              'basket_incentives_custody': '5996',
              'nebula_airdrop': '6009',
              'nebula_cluster': '5993',
              'nebula_cluster_factory': '6002',
              'nebula_collector': '6004',
              'nebula_community': '6008',
              'nebula_dummy_oracle': '6003',
              'nebula_gov': '5995',
              'nebula_incentives': '5991',
              'nebula_incentives_custody': '6005',
              'nebula_lp_staking': '5999',
              'nebula_penalty': '5998',
              'terraswap_factory': '5994',
              'terraswap_oracle': '5997',
              'terraswap_pair': '6000',
              'terraswap_router': '6007',
              'terraswap_token': '6001'},
  'collector': Contract("terra134x00e7qrap7eqlvaf3mqs96fcjk7stzqjkuad"),
  'community': Contract("terra1w207dpeysx6hdrqqkv68qeel67xxaldyge7x4j"),
  'dummy_oracle': None,
  'factory': Contract("terra1c88eegv4dsw0r3g05d8la6p6y2vg8gghkmhjlj"),
  'gov': Contract("terra1ccep3m8ph7vpr93k2l8cnc376qxd25e5ullsv6"),
  'incentives': Contract("terra1ww447wps8k26u44vljlvnqgx53w7sqa3us5pne"),
  'incentives_custody': Contract("terra1q90nlhfh9qmvgv6tlg33kd8ywz0crex09ghayh"),
  'lp_token': None,
  'neb_pair': Contract("terra17p5hushhk2jllfv9yhp55jrlz6ta7gt6ek7y3u"),
  'neb_token': Contract("terra1uquyhhpkju0359xryyh0c6rsgwplpcvr4shu9r"),
  'require_gov': True,
  'staking': Contract("terra1ghwfq5jpu8lajazjhqsddd5tcx6ep6s75p2fjt"),
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
    'ANC': Contract("terra15tecrcm27fenchxaqde9f8ws8krfgjnqf2hhcv"),
    'MIR': Contract("terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd"),
    'mAAPL': Contract("terra1pwd9etdemugqdt92t5d3g98069z0axpz9plnsk"),
    'mABNB': Contract("terra1jm4j6k0e2dpug7z0glc87lwvyqh40z74f40n52"),
    'mAMC': Contract("terra1wa87zjty4y983yyt604hdnyr8rm9mwz7let8uz"),
    'mAMZN': Contract("terra18mjauk9ug8y29q678c2qlee6rkd9aunrpe9q97"),
    'mBABA': Contract("terra1uvzz9fchferxpg64pdshnrc49zkxjcj66uppq8"),
    'mBTC': Contract("terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p"),
    'mCOIN': Contract("terra16e3xu8ly6a622tjykfuwuv80czexece8rz0gs5"),
    'mETH': Contract("terra1rxyctpwzqvldalafvry787thslne6asjlwqjhn"),
    'mFB': Contract("terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63"),
    'mGLXY': Contract("terra17sm265sez3qle769ef4hscx540wem5hvxztpxg"),
    'mGME': Contract("terra19y6tdnps3dsd7qc230tk3jplwl9jm27mpcx9af"),
    'mGOOGL': Contract("terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524"),
    'mGS': Contract("terra199yfqa5092v2udw0k0h9rau9dzel0jkf5kk3km"),
    'mIAU': Contract("terra1n7pd3ssr9sqacwx5hekxsmdy86lwlm0fsdvnwe"),
    'mMSFT': Contract("terra18aztjeacdfc5s30ms0558cy8lygvam3s4v69jg"),
    'mNFLX': Contract("terra1smu8dc2xpa9rfj525n3a3ttgwnacnjgr59smu7"),
    'mQQQ': Contract("terra1r20nvsd08yujq29uukva8fek6g32p848kzlkfc"),
    'mSLV': Contract("terra1re6mcpu4hgzs5wc77gffsluqauanhpa8g7nmjc"),
    'mSPY': Contract("terra1j3l2ul7s8fkaadwdan67hejt7k5nylmxfkwg0w"),
    'mTSLA': Contract("terra1k44gg67rnc6av8sn0602876w8we5lu3jp30yec"),
    'mTWTR': Contract("terra1897xd8jqjkfpr5496ur8n896gd8fud3shq3t4q"),
    'mUSO': Contract("terra1c3nyehgvukzrt5k9lxzzw64d68el6cejyxjqde"),
    'mVIXY': Contract("terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9")
 }

CONTRACT_TOKEN_TO_SYM_TEQ = {
    Contract("terra15tecrcm27fenchxaqde9f8ws8krfgjnqf2hhcv"): 'ANC',
    Contract("terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd"): 'MIR',
    Contract("terra1pwd9etdemugqdt92t5d3g98069z0axpz9plnsk"): 'mAAPL',
    Contract("terra1jm4j6k0e2dpug7z0glc87lwvyqh40z74f40n52"): 'mABNB',
    Contract("terra1wa87zjty4y983yyt604hdnyr8rm9mwz7let8uz"): 'mAMC',
    Contract("terra18mjauk9ug8y29q678c2qlee6rkd9aunrpe9q97"): 'mAMZN',
    Contract("terra1uvzz9fchferxpg64pdshnrc49zkxjcj66uppq8"): 'mBABA',
    Contract("terra13uya9kcnan6aevfgqxxngfpclqegvht6tfan5p"): 'mBTC',
    Contract("terra16e3xu8ly6a622tjykfuwuv80czexece8rz0gs5"): 'mCOIN',
    Contract("terra1rxyctpwzqvldalafvry787thslne6asjlwqjhn"): 'mETH',
    Contract("terra1xl2tf5sjzz9phm4veh5ty5jzqrjykkqw33yt63"): 'mFB',
    Contract("terra17sm265sez3qle769ef4hscx540wem5hvxztpxg"): 'mGLXY',
    Contract("terra19y6tdnps3dsd7qc230tk3jplwl9jm27mpcx9af"): 'mGME',
    Contract("terra1504y0r6pqjn3yep6njukehpqtxn0xdnruye524"): 'mGOOGL',
    Contract("terra199yfqa5092v2udw0k0h9rau9dzel0jkf5kk3km"): 'mGS',
    Contract("terra1n7pd3ssr9sqacwx5hekxsmdy86lwlm0fsdvnwe"): 'mIAU',
    Contract("terra18aztjeacdfc5s30ms0558cy8lygvam3s4v69jg"): 'mMSFT',
    Contract("terra1smu8dc2xpa9rfj525n3a3ttgwnacnjgr59smu7"): 'mNFLX',
    Contract("terra1r20nvsd08yujq29uukva8fek6g32p848kzlkfc"): 'mQQQ',
    Contract("terra1re6mcpu4hgzs5wc77gffsluqauanhpa8g7nmjc"): 'mSLV',
    Contract("terra1j3l2ul7s8fkaadwdan67hejt7k5nylmxfkwg0w"): 'mSPY',
    Contract("terra1k44gg67rnc6av8sn0602876w8we5lu3jp30yec"): 'mTSLA',
    Contract("terra1897xd8jqjkfpr5496ur8n896gd8fud3shq3t4q"): 'mTWTR',
    Contract("terra1c3nyehgvukzrt5k9lxzzw64d68el6cejyxjqde"): 'mUSO',
    Contract("terra12kt7yf3r7k92dmch97u6cu2fggsewaj3kp0yq9"): 'mVIXY'
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