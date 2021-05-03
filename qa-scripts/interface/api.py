from terra_sdk.core.wasm import dict_to_b64


class Oracle:
    @staticmethod
    def set_prices(prices):
        return {"set_prices": {"prices": prices}}


class CW20:
    @staticmethod
    def transfer(recipient, amount):
        return {"transfer": {"recipient": recipient, "amount": amount}}

    @staticmethod
    def send(contract, amount, msg_data=None):
        msg = None
        if msg_data is not None:
            msg = dict_to_b64(msg_data)
        return {"send": {"contract": contract, "amount": amount, "msg": msg}}


class Basket:
    @staticmethod
    def set_basket_token(basket_token):
        return {"__set_basket_token": {"basket_token": basket_token}}

    @staticmethod
    def mint(asset_amounts, min_tokens=None):
        return {"mint": {"asset_amounts": asset_amounts, "min_tokens": min_tokens}}

    @staticmethod
    def stage_asset():
        return {"stage_asset": {}}

    @staticmethod
    def burn(asset_weights=None, redeem_mins=None):
        return {
            "burn": {
                "asset_weights": asset_weights,
                "redeem_mins": redeem_mins,
                "random_fuckshit": None,
            }
        }


class Asset:
    @staticmethod
    def cw20_asset_info(haddr):
        return {"token": {"contract_addr": haddr}}

    @staticmethod
    def native_asset_info(denom):
        return {"native_token": {"denom": denom}}

    @staticmethod
    def asset(string, amount, native=False):
        if not native:
            return {"info": Asset.cw20_asset_info(string), "amount": amount}
        else:
            return {"info": Asset.native_asset_info(string), "amount": amount}
