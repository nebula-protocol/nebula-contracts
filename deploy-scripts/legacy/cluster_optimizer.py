# Alwin's method for optimizing a cluster

import os
import numpy as np
import math
from .strategy_base import StrategyBase
from interface.basket_logic import MinimumNotMetException
class BalancerStrat(StrategyBase):
    def __init__(self, basket, inventory, basket_tokens, graph=True):
        super().__init__(basket, inventory, basket_tokens, graph=graph)
    # the basket is unbalanced; there are some tokens that are under their target weight
    # what is the maximum number of those tokens we can add such that they remain <= their target weight in the basket?
    async def balance_into_bsk_notional(self):
        u = self.basket.target_weights * self.basket.asset_prices
        u /= u.sum()
        current_weights = self.basket.asset_prices * self.basket.asset_tokens
        basket_value = current_weights.sum()
        # can we add this much notional value into the basket such that we get a reward?
        # given some notional value, return a residual array arr, where arr[i] is the most that can be
        # added into asset i in a basket with that notional value, and have asset i not exceed
        # its weight in a completely balanced basket with the same notional value
        def get_addition(notional):
            new_notional = basket_value + notional
            goal = u * new_notional
            return np.core.umath.maximum(
                np.minimum(goal - current_weights, self.inv * self.basket.asset_prices),
                0,
            )
        lo = 0
        hi = int(np.dot(self.inv, self.basket.asset_prices))
        while lo < hi:
            mid = (lo + hi) // 2
            addition = get_addition(mid)
            tot = addition.sum()
            if tot > mid:
                lo = mid + 1
            else:
                hi = mid
        # why does this work?
        # let f(x) = sum(get_addition(x))
        # claim: f(0) > 0
        # proof: the basket is currently unbalanced, and thus there must be some things under their target weight
        # claim: f(x) is concave
        # proof: if you had unlimited inventory, f(x) would be c * x, since sum(goal) grows linearly with new_notional
        #        however, as notional grows higher, more of this growth gets caught by the minimum with self.inv * self.basket.asset_prices
        #        which makes the slope non-increasing (we ignore the spiky points)
        # claim: the optimal (maximum) assignment of notional value is at an x such that f(x) == x
        # proof: assume it is at an x such that f(x) > x. clearly this is not maximal because you can increase x and still have a valid assignment
        #        assume it is at an x such that f(x) < x. this assignment isn't valid, because an asset would have to either exceed its
        #        allotted weight or exceed your inventory
        # because of ^, we know that f(x) intersects with y == x at exactly one point where x > 0, and we can binary search
        # for that point depending on whether f(x) is is greater than or less than x.
        # TODO: why is this intersection point always on (0, hi]? is that even true?
        await self.submit_order(
            -(get_addition(lo) / self.basket.asset_prices).astype(np.int64)
        )
        # await self.execute(self.basket.asset_tokens + (get_addition(lo) / self.basket.asset_prices).astype(np.int64))
    async def redeem_out_of_bsk_notional(self):
        u = self.basket.target_weights * self.basket.asset_prices
        u /= u.sum()
        current_weights = self.basket.asset_prices * self.basket.asset_tokens
        basket_value = current_weights.sum()
        # there are some guidelines on how much we should be willing to redeem
        # for now let's say that if an asset token should make up x% of the notional
        # value in a basket, i do not let it exceed x% of the notional value of my
        # inventory
        notional_limits = self.notional * u
        max_redemption = np.core.umath.maximum(
            notional_limits - self.basket.asset_prices * self.inv, 0
        )
        def get_subtraction(notional):
            new_notional = basket_value - notional
            goal = u * new_notional
            return np.core.umath.maximum(
                np.minimum(current_weights - goal, max_redemption), 0
            )
        global_cap = int(self.basket_tokens * self.basket.token_value)
        lo = 0
        hi = global_cap
        while lo < hi:
            mid = (lo + hi + 1) // 2
            subtraction = get_subtraction(mid)
            tot = subtraction.sum()
            if tot >= mid:
                lo = mid
            else:
                hi = mid - 1
        # same logic as above, except the intersection point may not be on [0, global_cap) so some logic is needed to execute the bounds
        if lo:
            solu = get_subtraction(lo)
            if solu.sum() > global_cap:
                solu = solu / solu.sum() * global_cap
            await self.submit_order((solu / self.basket.asset_prices).astype(np.int64))
            # await self.execute(self.basket.asset_tokens - (solu / self.basket.asset_prices).astype(np.int64))
    async def smart_balance(self):
        await self.basket.sync()
        # none of these should depend on each other, the redeem and mint should both be
        # independently profitable, so ignore if there is some slippage issue
        try:
            await self.redeem_out_of_bsk_notional()
        except MinimumNotMetException:
            pass
        await self.basket.sync()
        try:
            await self.balance_into_bsk_notional()
        except MinimumNotMetException:
            pass
    async def tick(self):
        await self.smart_balance()