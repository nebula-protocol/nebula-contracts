import os
import sys

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio
from base import deployer
from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV

cluster_addr = sys.argv[1]
cluster = Contract(cluster_addr)

async def mint(cluster_state):
    assets = cluster_state['assets']
    cluster = Contract(cluster_state['cluster_contract_address'])
    
    types = [list(a.keys())[0] for a in assets]
    native_assets = []
    mint_cw20_assets = []
    for idx, t in enumerate(types):
        if 'native' in t:
            native_assets.append(assets[idx]['native_token']['denom'])
        else:
            mint_cw20_assets.append(assets[idx]['token']['contract_addr'])

    
    # Do this separately because we have a native asset
    msgs = []
    mint_assets = []
    send = {}
    for asset_info in native_assets:
        # Increase allowance of each
        mint_assets.append(Asset.asset(asset_info, "100000", native=True))
        send[asset_info] = "100000"

    for asset_info in mint_cw20_assets:
        # Increase allowance of each
        asset_contract = Contract(asset_info)
        msgs.append(asset_contract.increase_allowance(spender=cluster, amount="100000"))
        mint_assets.append(Asset.asset(asset_info, "100000"))

    # import pdb; pdb.set_trace()

    msgs.append(
        cluster.mint(asset_amounts=mint_assets, _send=send)
    )

    await chain(*msgs)

async def burn(cluster_state):
    assets = cluster_state['assets']
    cluster = Contract(cluster_state['cluster_contract_address'])

    cluster_token = Contract(cluster_state['cluster_token'])

    msgs = []

    msgs.append(cluster_token.increase_allowance(spender=cluster, amount="5000000"))

    msgs.append(
        cluster.burn(max_tokens="3000")
    )

    await chain(*msgs)

async def swap_ts(cluster_state):
    terraswap_factory = Contract('terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf')
    cluster_token = cluster_state['cluster_token']
    asset_infos = [Asset.cw20_asset_info(cluster_token), Asset.native_asset_info('uusd')] 
    pair_info = await terraswap_factory.query.pair(asset_infos=asset_infos)

    pair_contract = Contract(pair_info['contract_addr'])

    # Swap from uluna to cluster_token
    await pair_contract.swap(offer_asset=Asset.asset("uusd", amount="10000", native=True), _send={"uusd": "10000"},)


async def cluster_info():
    cfg = (await cluster.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])
    cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster_addr)
    return cluster_state

async def main():
    cluster_state = await cluster_info()
    print(cluster_state)

    cfg = (await cluster.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])

    prices = cluster_state['prices']
    assets = [list(list(asset.values())[0].values())[0] for asset in cluster_state['assets']]

    await oracle.set_prices(prices=list(zip(assets, prices)))
    
    # await swap_ts(cluster_state)
    # await mint(cluster_state)

    # cluster_state = await cluster_info()
    # print('After mint', cluster_state)

    await burn(cluster_state)
    
    cluster_state = await cluster_info()
    print('After burn', cluster_state)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
