import os
import sys
import numpy as np

os.environ["USE_BOMBAY"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio

factory_addr = sys.argv[1]
factory = Contract(factory_addr)

async def initial_mint(cluster_state):
    assets = [t['info'] for t in cluster_state['target']]
    amounts = [t['amount'] for t in cluster_state['target']]

    # How to scale up amounts for minting
    max_len = len(max(amounts, key=lambda a: len(a)))
    mult = max(12 - max_len, 0)

    cluster = Contract(cluster_state['cluster_contract_address'])
    
    types = [list(a.keys())[0] for a in assets]
    native_assets = []
    mint_cw20_assets = []
    for idx, t in enumerate(types):
        if 'native' in t:
            native_assets.append((assets[idx]['native_token']['denom'], amounts[idx]))
        else:
            mint_cw20_assets.append((assets[idx]['token']['contract_addr'], amounts[idx]))

    
    # Do this separately because we have a native asset
    msgs = []
    mint_assets = []
    send = {}
    amts = []

    for asset in native_assets:
        asset_info, amount = asset
        mint_amt = str(int(amount) * (10**mult))
        mint_assets.append(Asset.asset(asset_info, mint_amt, native=True))
        send[asset_info] = mint_amt
        amts.append(int(mint_amt))

    for asset in mint_cw20_assets:
        asset_info, amount = asset
        mint_amt = str(int(amount) * (10**mult))
        # Increase allowance of each
        asset_contract = Contract(asset_info)

        # Uncomment if large message
        # await asset_contract.increase_allowance(spender=cluster, amount=mint_amt)

        msgs.append(asset_contract.increase_allowance(spender=cluster, amount=mint_amt))
        mint_assets.append(Asset.asset(asset_info, mint_amt))
        amts.append(int(mint_amt))

    prices = np.array([float(i) for i in cluster_state['prices']])
    amts = np.array(amts)
    min_tokens = str(int(np.dot(prices, amts)))

    print('Mint assets', mint_assets)

    # Want each token to be roughly $1
    msgs.append(
        cluster.rebalance_create(asset_amounts=mint_assets, min_tokens=min_tokens, _send=send)
    )

    # await cluster.mint(asset_amounts=mint_assets, min_tokens=min_tokens, _send=send)
    await chain(*msgs)
    print('Done with mint')

def cost_per_cluster_token(cluster_state):
    outstanding = int(cluster_state['outstanding_balance_tokens'])
    inv = np.array([int(i) for i in cluster_state['inv']])
    prices = np.array([float(i) for i in cluster_state['prices']])
    notional_val = np.dot(inv, prices)
    print(notional_val)
    print(outstanding)
    return notional_val / outstanding


async def mint_and_provide(cluster):
    cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster, stale_threshold=0)
    cluster_info = await cluster.query.cluster_info()
    print(cluster_info)
    print(cluster_state)

    if cluster_info['name'] == 'Terra Ecosystem' or cluster_info['name'] == 'The Metaverse':
        return
    
    if cluster_state['outstanding_balance_tokens'] == '0':
        await initial_mint(cluster_state)
        cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster, stale_threshold=0)
        print('After initial mint cluster state', cluster_state)

    cluster_token = Contract(cluster_state['cluster_token'])
    terraswap_factory = Contract('terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf')

    asset_infos = [Asset.cw20_asset_info(cluster_token), Asset.native_asset_info('uusd')] 
    pair_info = await terraswap_factory.query.pair(asset_infos=asset_infos)

    pair_contract = Contract(pair_info['contract_addr'])

    msgs = []

    provide_uusd = 1000000000000 # Provide $500 liquidity

    cost_per_ct = cost_per_cluster_token(cluster_state)
    provide_cluster_token = int(provide_uusd / cost_per_ct)
    print(provide_cluster_token)
    # Increase allowance
    msgs.append(cluster_token.increase_allowance(spender=pair_contract, amount=str(provide_cluster_token)))

    # Provide liquidity
    assets = [Asset.asset(cluster_token, str(provide_cluster_token)), Asset.asset('uusd', str(provide_uusd), native=True)]
    msgs.append(pair_contract.provide_liquidity(assets=assets, _send={"uusd": str(provide_uusd)}))
    await chain(*msgs)

    print(f'Done providing liquidity with {provide_cluster_token} cluster tokens')

async def main():
    clusters = (await factory.query.cluster_list())['contract_infos']
    cluster_contracts = [Contract(c[0]) for c in clusters]
    for c in cluster_contracts:
        await mint_and_provide(c)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
