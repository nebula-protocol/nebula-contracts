import numpy as np
import asyncio
import math

from interface.basket_logic import BasketLogic


class StrategyBase(object):
    def __init__(self, interface, inventory, basket_tokens, graph=False):

        if isinstance(interface, BasketLogic):
            from interface.local import InterfaceLocal

            interface = InterfaceLocal(interface)

        self.basket = interface
        self.inv = np.array(inventory, dtype=np.int64)
        self.basket_tokens = int(basket_tokens)
        self.graph = graph

    def fork(self):
        return self.__class__(self.basket.fork(), self.inv, self.basket_tokens)

    async def mint(self, amounts, min_tokens=None):
        # allow infinite inventory strategies
        amount = await self.basket.mint(amounts, min_tokens=min_tokens)
        assert (amounts >= 0).all()
        self.basket_tokens += amount
        if self.inv is not None:
            self.inv -= amounts
            if not (self.inv >= 0).all():
                raise Exception("MINT UHOH")
        return amount

    async def redeem(self, amount, weights=None, min_tokens=None):
        redeemed = await self.basket.redeem(
            amount, weights=weights, min_tokens=min_tokens
        )
        if weights is not None:
            assert (weights >= 0).all()
        assert amount >= 0
        if self.inv is not None:
            self.inv += redeemed
        self.basket_tokens -= amount
        if not self.basket_tokens >= 0:
            raise Exception("REDEEM UHOH")
        return redeemed

    async def submit_order(self, order, slippage="profit", check_profit=True):

        assert slippage in {"profit", "exact"}
        # profit - minimum return such that the notional value of this inventory increases
        # exact - only allow this exact trade

        # order is a numpy array where order[i] is the desired change in my inventory for asset i
        if (order > 0).any() and (order < 0).any():
            raise Exception(f"Invalid order {order} requires mint and redeem")

        if (order == 0).all():
            return True

        mint = (order < 0).any()

        forked = self.fork()
        current_notional = forked.notional

        if mint:

            if ((self.inv + order) < 0).any():
                raise Exception(
                    f"Attempting to mint with order {order} when not enough coins in inventory {target_basket_inventory}"
                )

            expected = await forked.mint(-order)
            if check_profit and forked.notional < current_notional:
                return False

            min_tokens = (
                expected
                if slippage == "exact"
                else math.ceil(
                    np.dot(-order, self.basket.asset_prices) / self.basket.token_value
                )
            )
            await self.mint(-order, min_tokens=min_tokens)

        else:
            need_tokens = (
                self.basket.basket_tokens
                * np.dot(order, self.basket.asset_prices)
                / self.basket.notional
            )
            need_tokens = math.ceil(need_tokens)

            if need_tokens > self.basket_tokens:
                raise Exception(
                    f"Attempting redeem op {order} which requires {need_tokens} basket tokens when inventory only has {self.basket_tokens}"
                )

            expected = await forked.redeem(need_tokens, weights=order)

            if check_profit and forked.notional < current_notional:
                return False

            if slippage == "exact":
                min_tokens = expected
            else:
                in_value = self.basket.token_value * need_tokens
                current_sum = np.dot(order, self.basket.asset_prices)

                lo = 0
                hi = 1e9 + 7

                while (
                    (lo * order).astype(np.int64) != (hi * order).astype(np.int64)
                ).any():

                    mid = (lo + hi) / 2

                    # limits of precision... just exit and go high
                    if mid == lo or mid == hi:
                        break
                    if (
                        np.dot(self.basket.asset_prices, (mid * order).astype(np.int64))
                        < in_value
                    ):
                        lo = mid
                    else:
                        hi = mid
                min_tokens = (hi * order).astype(np.int64)
            await self.redeem(need_tokens, weights=order, min_tokens=min_tokens)

        return True

    async def submit_order_chain(self, orders, check_profit=True):
        pass

    async def tick(self):
        raise NotImplementedError

    async def main_loop(self, delay=0):
        while True:
            await self.tick()
            await asyncio.sleep(delay)

    def summary(self):
        return {
            "basket_tokens": self.basket_tokens,
            "inventory": self.inv.tolist(),
            "notional": self.notional,
        }

    @property
    def notional(self):
        return self.basket.token_value * self.basket_tokens + np.dot(
            self.basket.asset_prices, self.inv
        )
