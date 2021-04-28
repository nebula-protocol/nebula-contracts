from interface.basket_logic import BasketLogic
from interface.live import InterfaceLive
from interface.local import InterfaceLocal
from interface.utils import terra
import asyncio
import time
from interface.utils import create_basket
from random import randint, random


BASKET_PARAMS = {
    "basket_tokens": 1000,
    "asset_tokens": [5000, 5000],
    "asset_prices": [1, 1],
    "target_weights": [1, 1],
    "penalty_params": {"a_neg": 0.1, "a_pos": 0.5, "s_neg": 0.3, "s_pos": 0.5},
}


def find_issues(mode="mint"):
    for _ in range(10000):

        params = {
            "basket_tokens": randint(1, 1000000),
            "asset_tokens": [randint(1, 1000000)] * 2,
            "asset_prices": [round(random() * 1000000, 2) for _ in range(2)],
            "target_weights": [1, 1],
            "penalty_params": {"a_neg": 0.1, "a_pos": 0.5, "s_neg": 0.3, "s_pos": 0.5},
        }

        logic = BasketLogic(**params)

        if mode == "mint":
            logic.mint(params["asset_tokens"])
            if logic.basket_tokens != params["basket_tokens"] * 2:
                print(params)
                continue
        elif mode == "redeem":
            logic.redeem(params["basket_tokens"])
            if (logic.asset_tokens != 0).any():
                print(params)
        else:
            raise Exception("Invalid mode")


async def test_double_mint(basket_params):
    if live:
        basket, basket_token, assets = await create_basket(**basket_params)
        interface = InterfaceLive(basket, basket_token, assets)
    else:
        interface = InterfaceLocal(BasketLogic(**basket_params))

    tokens = await interface.mint(basket_params["asset_tokens"])
    assert tokens == basket_params["basket_tokens"]


find_issues()
