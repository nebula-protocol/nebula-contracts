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

async def initial_mint(cluster_state):
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
        mint_assets.append(Asset.asset(asset_info, "1000000000", native=True))
        send[asset_info] = "1000000000"

    for asset_info in mint_cw20_assets:
        # Increase allowance of each
        asset_contract = Contract(asset_info)
        msgs.append(asset_contract.increase_allowance(spender=cluster, amount="1000"))
        mint_assets.append(Asset.asset(asset_info, "1000"))

    msgs.append(
        cluster.mint(asset_amounts=mint_assets, min_tokens="100", _send=send)
    )

    await chain(*msgs)



async def cluster_info():
    cfg = (await cluster.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])
    cluster_state = await cluster.query.cluster_state(cluster_contract_address=cluster_addr)
    return cluster_state

async def main():
    cluster_state = await cluster_info()
    print(cluster_state)
    if cluster_state['outstanding_balance_tokens'] == '0':
        await initial_mint(cluster_state)

    cluster_token = Contract(cluster_state['cluster_token'])
    terraswap_factory = Contract('terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf')

    asset_infos = [Asset.cw20_asset_info(cluster_token), Asset.native_asset_info('uusd')] 
    pair_info = await terraswap_factory.query.pair(asset_infos=asset_infos)

    pair_contract = Contract(pair_info['contract_addr'])

    # Increase allowance
    await cluster_token.increase_allowance(spender=pair_contract, amount="20")

    # Provide liquidity
    assets = [Asset.asset(cluster_token, "20"), Asset.asset('uusd', "10000000", native=True)]
    await pair_contract.provide_liquidity(assets=assets, _send={"uusd": "10000000"},)

    print('Done providing liquidity')



if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
