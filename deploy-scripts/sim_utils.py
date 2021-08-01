from api import Asset
from contract_helpers import Contract, chain
from constants import terraswap_factory_teq

def unpack_asset_info(asset_info):
    if asset_info.get('native_token'):
        return asset_info['native_token']['denom'], True
    else:
        return asset_info['token']['contract_addr'], False

def unpack_asset(asset):
    asset_name, is_native = unpack_asset_info(asset['info'])
    return [asset_name, is_native, int(asset['amount'])]

# Write function to return USD-token pair contract
async def get_pair_contract_uusd(asset, price=None, tequila=True):

    if not tequila:
        raise NotImplementedError

    asset_name, is_native, amount = unpack_asset(asset)

    if is_native:
        query_asset_info = {
            "native_token": {
                "denom": asset_name
            }
        }
    else:
        query_asset_info = {
            "token": {
                "contract_addr": asset_name
            }
        }

    query_pair = [
        query_asset_info,
        {
            "native_token": {
                "denom": "uusd"
            }
        }
    ]
    try:
        pair_contract = (await terraswap_factory_teq.query.pair(asset_infos=query_pair))['contract_addr']
        return pair_contract
    except:
        print('Creating pair for cw20 token')

        if price is None:
            raise ValueError

        if is_native:
            raise NotImplementedError

        await terraswap_factory_teq.create_pair(asset_infos=query_pair)

        print('Pair contract successfully created')

        pair_contract = (await terraswap_factory_teq.query.pair(asset_infos=query_pair))['contract_addr']

        print('New pair contract successfully queried', pair_contract)

        await provide_liquidity(asset, price)

        print('Liquidity provided to new pair')

        return pair_contract

async def provide_liquidity(asset, price):
    pair_contract = Contract(await get_pair_contract_uusd(asset))
    msgs = []        

    provide_uusd = 100000000
    provide_token = int(provide_uusd / price)
    
    asset_name, is_native, amount = unpack_asset(asset)
    token_contract = Contract(asset_name)

    # Increase allowance
    msgs.append(token_contract.increase_allowance(spender=pair_contract, amount=str(provide_token)))

    # Provide liquidity
    assets = [Asset.asset(token_contract, str(provide_token)), Asset.asset('uusd', str(provide_uusd), native=True)]
    msgs.append(pair_contract.provide_liquidity(assets=assets, _send={"uusd": str(provide_uusd)}))
    await chain(*msgs)

