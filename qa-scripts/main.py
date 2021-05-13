from random import randint
from interface.basket_logic import BasketLogic
from interface.local import InterfaceLocal
from interface.live import InterfaceLive
from strategy.retail.normal import NormalFlow
from strategy.sim import Simulator
from interface.utils import create_basket


BASKET_INFO = (
    "terra17zlr63su47t6njxw6xxnd60g37gx5d2wl5cf3a",
    "terra1axal75wfknd495he2l7sk4z5w7nttkxer9596c",
    (
        "terra10v4sz5v9azqulsrhrsrltafj0pg32ha3k0kh0a",
        "terra12f7kkeeu0kctfna3usgz38usth2csuwa798jke",
    ),
)


async def run_strategies():

    basket = BasketLogic(
        20000000,
        [1000000, 1000000],
        [1, 1],
        [1, 1],
        {
            "penalty_amt_lo": "0.1",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "0.5",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.05",
            "reward_cutoff": "0.02",
        },
    )
    live = False

    def get():
        if live:
            return InterfaceLive(*BASKET_INFO)
        return basket

    strat0 = NormalFlow(get(), threshold=0.95)

    strat1 = NormalFlow(get(), threshold=0.95)
    strat2 = NormalFlow(get(), threshold=0.95)

    s = Simulator(
        basket=basket,
        strategies=[strat1, strat2],
        update_prices=False,
    )
    await s.go_sync(ticks=100000)


async def tiny_basket():
    basket_args = await create_basket(
        50,
        [0, 5000],
        [1, 1],
        [1, 1],
        {
            "penalty_amt_lo": "0.1",
            "penalty_cutoff_lo": "0.01",
            "penalty_amt_hi": "0.5",
            "penalty_cutoff_hi": "0.1",
            "reward_amt": "0.05",
            "reward_cutoff": "0.02",
        },
    )

    interface = InterfaceLive(*basket_args)
    await interface.mint([10, 10], min_tokens=300)
    await interface.sync()
    print(interface.summary())

    await interface.mint([100000, 100000])
    await interface.sync()
    print(interface.summary())

    for _ in range(50):
        await interface.mint([10, 10])
        await interface.sync()
        print(interface.summary())
        await asyncio.sleep(10)

    # print(await interface.redeem(50))


if __name__ == "__main__":

    import asyncio

    loop = asyncio.get_event_loop()
    loop.run_until_complete(tiny_basket())
