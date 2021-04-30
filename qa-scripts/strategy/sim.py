import numpy as np
import random
import asyncio


class Simulator:
    def __init__(self, basket, strategies, update_prices=False):

        self.basket = basket
        self.strategies = strategies
        self.update_prices = update_prices
        self.done = False

    async def supervisor(self):
        results = {idx: [] for idx, i in enumerate(self.strategies) if i.graph}
        try:
            while True:
                if self.update_prices and random.random() < 0.1:
                    self.basket.asset_prices *= np.random.choice(
                        [1.01, 1 / 1.01], len(self.basket.asset_prices), p=[0.5, 0.5]
                    )
                for idx, strategy in enumerate(self.strategies):
                    if strategy.graph:
                        print(strategy.summary())
                        results[idx].append(strategy.notional)

                print("T", self.strategies[0].basket.token_value)
                print(self.strategies[0].basket.summary())
                print("-=" * 50)
                await asyncio.sleep(1)
        except asyncio.CancelledError:
            return results

    async def go_sync(self, ticks):

        for _ in range(ticks):
            for strat in self.strategies:
                await strat.tick()
                if strat.graph:
                    print(strat.summary())
            print(self.strategies[0].basket.summary())
            print("=-" * 20)

    async def go(self, seconds=5, delays=None):

        if delays is None:
            delays = [0] * len(self.strategies)
        await asyncio.gather(*[i.basket.sync() for i in self.strategies])
        supervisor = asyncio.create_task(self.supervisor())
        tasks = [
            strat.main_loop(delay) for strat, delay in zip(self.strategies, delays)
        ]

        try:
            await asyncio.wait_for(asyncio.gather(*tasks), timeout=seconds)
        except asyncio.TimeoutError:
            pass

        supervisor.cancel()
        results = await supervisor

        import matplotlib.pyplot as plt

        for idx, result in results.items():
            plt.plot(result, label=str(idx))
        plt.legend(loc="best")
        plt.savefig("result.png")
        self.done = True
