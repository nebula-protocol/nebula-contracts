class Asset:
    @staticmethod
    def cw20_asset_info(haddr):
        return {"token": {"contract_addr": haddr}}

    @staticmethod
    def native_asset_info(denom):
        return {"native_token": {"denom": denom}}

    @staticmethod
    def asset_info(param):
        if param == "uluna" or param == 'uusd':
            return Asset.native_asset_info(param)
        return Asset.cw20_asset_info(param)

    @staticmethod
    def asset(string, amount, native=False):
        if not native:
            return {"info": Asset.cw20_asset_info(string), "amount": str(amount)}
        else:
            return {"info": Asset.native_asset_info(string), "amount": str(amount)}
