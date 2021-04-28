from .base import InterfaceBase
from .basket_logic import BasketLogic
import random
import asyncio
import numpy as np


# points to a BasketLogic which simulates a basket smart contract running locally
class InterfaceLocal(InterfaceBase):
    def __init__(self, local_basket, delay_avg=0, delay_range=0):
        super().__init__()

        self.delay_avg = delay_avg
        self.delay_range = delay_range

        self.__basket = local_basket
        self.main = self.__basket.main
        self._sync()

    def fork(self):
        logic = BasketLogic.from_interface(self)
        return InterfaceLocal(logic)

    def get_delay(self):
        return asyncio.sleep(
            max(0, self.delay_avg + (2 * random.random() - 1) * self.delay_range)
        )

    def _sync(self):

        # need copies otherwise references lying about get written to...
        self.basket_tokens = self.__basket.basket_tokens
        self.asset_tokens = np.array(self.__basket.asset_tokens)
        self.asset_prices = np.array(self.__basket.asset_prices)
        self.target_weights = np.array(self.__basket.target_weights)
        self.penalty_params = self.__basket.penalty_params

    async def sync(self):
        await self.get_delay()
        self._sync()

    async def mint(self, amounts, min_tokens=None):
        await self.get_delay()
        return self.__basket.mint(amounts, min_tokens=min_tokens)

    async def redeem(self, amounts, weights=None, min_tokens=None):
        await self.get_delay()
        return self.__basket.redeem(amounts, weights=weights, min_tokens=min_tokens)

    def _internal_summary(self):
        return self.__basket.summary()
