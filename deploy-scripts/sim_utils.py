from api import Asset

def unpack_asset_info(asset_info):
    if asset_info.get('native_token'):
        return asset_info['native_token']['denom'], True
    else:
        return asset_info['token']['contract_addr'], False

def unpack_asset(asset):
    asset_name, is_native = unpack_asset_info(asset['info'])
    return [asset_name, is_native, int(asset['amount'])]

