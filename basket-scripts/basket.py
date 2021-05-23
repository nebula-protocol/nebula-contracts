from terra_sdk.core.wasm import dict_to_b64


class Oracle:
    @staticmethod
    def set_prices(prices):
        return {"set_prices": {"prices": prices}}

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
    def burn(max_tokens, asset_amounts=None):
        return {"burn": {"max_tokens": max_tokens, "asset_amounts": asset_amounts}}

    @staticmethod
    def stage_native_asset(denom, amount):
        return {"stage_native_asset": {"asset": Asset.asset(denom, amount, native=True)}}

    @staticmethod
    def reset_target(new_assets, new_target):
        return {"reset_target": {"assets": new_assets, "target": new_target}}

    @staticmethod
    def reset_penalty(penalty_contract):
        return {"reset_penalty": {"penalty": penalty_contract}}

    @staticmethod
    def reset_composition_oracle(penalty_contract):
        return {"reset_composition_oracle": {"composition_oracle": penalty_contract}}

    @staticmethod
    def reset_owner(owner):
        return {"__reset_owner": {"owner": owner}}

class Governance:
    @staticmethod
    def create_poll(title, description, link = None, execute_msg=None):
        return {
            "create_poll": {
                "title": title, 
                "description": description, 
                "link": link, 
                "execute_msg": execute_msg
            }
        }

    def create_execute_msg(contract, msg_data=None):
        msg = None
        if msg_data is not None:
            msg = dict_to_b64(msg_data)
        return {"contract": contract, "msg": msg}
    
    @staticmethod
    def stake_voting_tokens():
        return {"stake_voting_tokens": {}}

    @staticmethod
    def cast_vote(poll_id, vote, amount):
        assert (vote == "yes" or vote == "no")
        return {"cast_vote": {"poll_id": poll_id, "vote": vote, "amount": amount}}

    @staticmethod
    def execute_poll(poll_id):
        return {"execute_poll": {"poll_id": poll_id}}

    @staticmethod
    def end_poll(poll_id):
        return {"end_poll": {"poll_id": poll_id}}

class Community:
    @staticmethod
    def spend(recipient, amount):
        return {"spend": { "recipient": recipient, "amount": amount }}

class Airdrop:
    @staticmethod
    def register_merkle_root(root):
        return { "register_merkle_root": { "merkle_root": root }}

    @staticmethod
    def claim(stage, amount, proof):
        return {
            "claim": {
                "stage": 1,
                "amount": amount,
                "proof": proof
            }
        }