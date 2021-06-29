# File to show how current cross_weighting algorithm can weight a marked asset higher than unmarked

# Intuition is that weight penalty for a marked asset is floored by X while there exists no floor 
# for an unmarked asset given existence of other unmarked assets

# This means that a marked asset will decrease by X% at most while an unmarked asset can go to 0

THRESHOLD = 0.5
X = 0.2

def cross_weighting():
    has_cross, all_cross = True, False
    best_pwps = {'mFB': 0.51, 'mGOOG': 0.74, 'mAAPL': 0.75}
    non_cross_assets = ['mTSLA']
    asset_names = ['mFB', 'mGOOG', 'mAAPL', 'mTSLA']
    # Calculate best_pwps and assets with crosses
    asset_data = {
        'mFB': 10, 
        'mTSLA': 10, 
        'mGOOG': 10,
        'mAAPL': 10,
    }
    asset_mcaps = list(asset_data.values())
    # All assets have crosses
    if all_cross:
        diffs = [best_pwps[asset] - THRESHOLD for asset in asset_names]
        denom = sum([asset_mcaps[i] * diffs[i] for i in range(len(asset_names))])
        target = {asset_names[i]: (asset_mcaps[i] * diffs[i])/denom for i in range(len(asset_names))}
    else:
        denom = sum(asset_mcaps)
        target = {asset_names[i]: asset_mcaps[i]/denom for i in range(len(asset_names))}
        print("Original allocation: {}".format(target))
        # Some asset has a cross, then divide up X share of non-cross assets to cross assets
        if has_cross:
            non_cross_pool = 0
            for asset_name in non_cross_assets:
                share = X * target[asset_name]
                non_cross_pool += share
                target[asset_name] -= share
                print("{} does not have a cross. Pool takes {}".format(asset_name, share))
            print("Total Non-Cross Pool: {}".format(non_cross_pool))
            cross_assets = list(best_pwps.keys())
            cross_diffs = [best_pwps[asset] - THRESHOLD for asset in cross_assets]
            cross_asset_mcaps = [asset_data[asset] for asset in cross_assets]
            denom = sum([cross_asset_mcaps[i] * cross_diffs[i] for i in range(len(cross_assets))])
            for i in range(len(cross_assets)):
                cross_asset = cross_assets[i]
                new_weight = (cross_asset_mcaps[i] * cross_diffs[i])/denom
                target[cross_asset] = new_weight
                print("{} target weight updated to {}".format(cross_asset, new_weight))
    assets, target_weights = zip(*target.items())
    return list(assets), list(target_weights)

import requests
import json
import pandas as pd
import asyncio
from graphql_querier import mirror_history_query

def graphql_query():


    # query = """
    # query {{
    #     asset(token: "terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz") {{
    #         prices {{
    #         history(interval: {0}, from: {1}, to: {2}) {{
    #             timestamp
    #             price
    #         }}
    #         }},
    #         symbol,
    #         name
    #     }}
    # }}""".format("5", "1624932000000", "1624932929000")

    # url = 'https://graph.mirror.finance/graphql'
    # r = requests.post(url, json={'query': query})
    # print(r.status_code)

    # import pdb; pdb.set_trace()
    # print(r.text)

    # query = """
    # query {{
    #     asset(token: "terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz") {{
    #         prices {{
    #         history(interval: {0}, from: {1}, to: {2}) {{
    #             timestamp
    #             price
    #         }}
    #         }},
    #         symbol,
    #         name
    #     }}
    # }}""".format("5", "1624932000000", "1624932929000")

    # url = 'https://graph.mirror.finance/graphql'
    # r = requests.post(url, json={'query': query})
    # print(r.status_code)

    # import pdb; pdb.set_trace()
    # print(r.text)


    test = mirror_history_query('\"terra1vxtwu4ehgzz77mnfwrntyrmgl64qjs75mpwqaz\"', "5", "1624932000000", "1624932929000")
    print(test)


if __name__ == "__main__":
    graphql_query()