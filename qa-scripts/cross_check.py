from interface.live import InterfaceLive
from interface.local import InterfaceLocal
from interface.basket_logic import BasketLogic
from interface.utils import create_basket
from random import randint
import numpy as np


async def create_and_test(basket_args, ops):

    local_basket = BasketLogic(**basket_args)
    local = InterfaceLocal(local_basket)

    live_basket_args = await create_basket(**basket_args)
    live = InterfaceLive(*live_basket_args)

    print("Created basket with params", basket_args)

    for op_type, amt in ops:
        print(f"{op_type} with {amt}")
        if op_type == "mint":
            recv_local = await local.mint(amt)
            recv_live = await live.mint(amt)
            assert recv_local == recv_live, f"Local minted {recv_local} but live minted {recv_live}"

        elif op_type == "redeem":
            recv_local = await local.redeem(amt)
            recv_live = await live.redeem(amt)

            assert (recv_local == recv_live).all(), f"Local redeemed {recv_local} but live redeemed {recv_live}"


        print("Basket state: ", local_basket.summary())


basket_params = {
    "basket_tokens": 20000,
    "asset_tokens": [10000, 10000],
    "asset_prices": [1, 1],
    "target_weights": [1, 1],
    "penalty_params": {"a_neg": 0.1, "a_pos": 0.5, "s_neg": 0.3, "s_pos": 0.5},
}

ops = [
    ["mint", [100, 100]],
    ["redeem", 200],
    ["mint", [3000, 1500]],
    ["redeem", 10000]
]

import asyncio
asyncio.get_event_loop().run_until_complete(create_and_test(basket_params, ops))