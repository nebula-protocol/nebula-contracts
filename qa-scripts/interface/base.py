import numpy as np


class InterfaceBase:
    def __init__(self):
        self.basket_tokens = None
        self.asset_tokens = None
        self.asset_prices = None
        self.target_weights = None
        self.penalty_contract = None
        self.penalty_params = None

    @classmethod
    async def create(cls, *args, **kwargs):
        interface = cls(*args, **kwargs)
        await interface.sync()
        return interface

    async def fork(self):
        raise NotImplementedError

    async def sync(self):
        raise NotImplementedError

    async def mint(self, amounts, min_tokens=None):
        raise NotImplementedError

    async def redeem(self, amount, weights=None, min_tokens=None):
        raise NotImplementedError

    @property
    def notional(self):
        return np.dot(self.asset_tokens, self.asset_prices)

    @property
    def token_value(self):
        return self.notional / self.basket_tokens

    def summary(self):
        return {
            "basket_tokens": self.basket_tokens,
            "asset_tokens": self.asset_tokens,
            "asset_prices": self.asset_prices,
            "target_weights": self.target_weights,
        }
