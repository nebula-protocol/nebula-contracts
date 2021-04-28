from random import randint
from interface.basket_logic import BasketLogic
from interface.local import InterfaceLocal
from interface.live import InterfaceLive
from strategy.balancer import BalancerStrat
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
        {"a_neg": 0.1, "a_pos": 0.5, "s_neg": 0.3, "s_pos": 0.5},
    )
    live = True

    def get():
        if live:
            return InterfaceLive(*BASKET_INFO)
        return basket

    strat0 = BalancerStrat(get(), [1000, 1000], 0)

    strat1 = BalancerStrat(get(), [1000, 1000], 0)
    strat2 = NormalFlow(get(), threshold=0.95)

    s = Simulator(
        basket=basket,
        strategies=[strat1, strat2],
        update_prices=False,
    )
    await s.go_sync(ticks=10)


async def tiny_basket():
    basket_args = await create_basket(
        50,
        [0, 5000],
        [1, 1],
        [1, 1],
        {"a_neg": 0.1, "a_pos": 0.5, "s_neg": 0.3, "s_pos": 0.5},
    )

    interface = InterfaceLive(*basket_args)
    await interface.sync()
    print(interface.summary())
    print(await interface.redeem(50))


if __name__ == "__main__":

    import asyncio

    loop = asyncio.get_event_loop()
    loop.run_until_complete(run_strategies())
