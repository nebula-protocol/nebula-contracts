import numpy as np
import concurrent.futures
import functools
import ast
import asyncio

from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64


from .base import InterfaceBase
from .basket_logic import BasketLogic
from .local import InterfaceLocal
from .api import Oracle, Basket, CW20, Asset
from . import utils
from .utils import deployer, terra, seq


# live interface points to an actual contract running on terra
class InterfaceLive(InterfaceBase):
    def __init__(self, basket, basket_token, assets):
        super().__init__()
        self.basket = basket
        self.basket_token = basket_token
        self.assets = assets

        self.penalty_param_cache = {}

    def fork(self):
        logic = BasketLogic.from_interface(self)
        return InterfaceLocal(logic)

    async def sync(self):
        basket_state = await terra.wasm.contract_query(
            self.basket, {"basket_state": {"basket_contract_address": self.basket}}
        )
        self.basket_tokens = int(basket_state["outstanding_balance_tokens"])
        self.asset_prices = np.array(basket_state["prices"], dtype=np.longdouble)
        self.asset_tokens = np.array(basket_state["inv"], dtype=np.int64)
        self.target_weights = np.array(basket_state["target"], dtype=np.int64)
        self.penalty_contract = basket_state["penalty"]

        if self.penalty_contract not in self.penalty_param_cache:
            penalty_state = await terra.wasm.contract_query(
                self.penalty_contract, {"params": {}}
            )
            self.penalty_param_cache[self.penalty_contract] = {
                k: np.longdouble(v) for k, v in penalty_state["penalty_params"].items()
            }

        self.penalty_params = self.penalty_param_cache[self.penalty_contract]

    async def mint(self, amounts, min_tokens=None):

        amounts = [str(i) for i in amounts]
        if min_tokens is not None:
            min_tokens = str(min_tokens)

        assets = []
        transfer_tokens_msg = []
        for asset, amount in zip(self.assets, amounts):
            if amount != "0":
                transfer_tokens_msg.append(
                    MsgExecuteContract(
                        deployer.key.acc_address,
                        asset,
                        CW20.send(self.basket, amount, Basket.stage_asset()),
                    ),
                )
            assets.append(Asset.asset(asset, amount))

        transfer_tokens_msg.append(
            MsgExecuteContract(
                deployer.key.acc_address,
                self.basket,
                Basket.mint(assets, min_tokens=min_tokens),
            ),
        )

        stage_and_mint_tx = await deployer.create_and_sign_tx(
            msgs=transfer_tokens_msg,
            sequence=seq(),
            fee=StdFee(4000000, "2000000uusd"),
        )

        result = await terra.tx.broadcast(stage_and_mint_tx)

        if result.is_tx_error():
            raise Exception(result.raw_log)

        mint_log = result.logs[-1].events_by_type

        import pprint

        pprint.pprint(mint_log)

        mint_total = mint_log["from_contract"]["mint_total"][0]
        return int(mint_total)

    async def redeem(self, amount, weights=None, min_tokens=None):

        amount = str(amount)
        if weights is not None:
            weights = [str(i) for i in weights]

        if min_tokens is not None:
            min_tokens = [
                Asset.asset(asset, str(mn))
                for asset, mn in zip(self.assets, min_tokens)
            ]

        result = await utils.execute_contract(
            self.basket_token,
            CW20.send(
                self.basket,
                amount,
                Basket.burn(
                    [
                        Asset.asset(asset, weight)
                        for asset, weight in zip(self.assets, weights)
                    ]
                    if weights
                    else None,
                    redeem_mins=min_tokens,
                ),
            ),  # asset weights must be integers
            fee=StdFee(
                4000000, "20000000uusd"
            ),  # burning may require a lot of gas if there are a lot of assets
        )

        if result.is_tx_error():
            raise Exception(result.raw_log)

        redeem_log = result.logs[0].events_by_type

        import pprint

        for thing in result.logs:
            pprint.pprint(thing.events_by_type)

        redeem_totals = redeem_log["from_contract"]["redeem_totals"]
        return ast.literal_eval(redeem_totals[0])
