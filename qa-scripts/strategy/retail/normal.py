import random
from ..strategy_base import StrategyBase
import numpy as np


class NormalFlow(StrategyBase):
    def __init__(self, interface, threshold=0.95):

        super().__init__(interface, [], 0, graph=False)
        self.threshold = threshold
        self.inv = None

    async def tick(self):
        await self.basket.sync()
        while True:
            if random.random() < 0.5:
                # attempt to redeem
                redeem_amt = random.randint(1, 100) * self.basket_tokens // 100
                if self.basket.basket_tokens - 100 >= redeem_amt and redeem_amt > 0:
                    await self.redeem(redeem_amt)
                    break
                else:
                    continue
            else:
                deposit_amt = [
                    random.randint(0, 10000)
                    for _ in range(len(self.basket.asset_tokens))
                ]

                if not any(deposit_amt):
                    continue

                deposit_val = sum(
                    a * b for a, b in zip(deposit_amt, self.basket.asset_prices)
                )
                sim = self.basket.fork()
                amt = await sim.mint(deposit_amt)

                # say the retail flow only deposits if they don't lose more than 2%

                if amt * sim.token_value > self.threshold * deposit_val:
                    await self.mint(np.array(deposit_amt))
                    break
                else:
                    continue
