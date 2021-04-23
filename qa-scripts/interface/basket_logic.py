import numpy as np
import math

DEBUG = False


class MinimumNotMetException(Exception):
    pass


class BasketLogic:
    def __init__(
        self,
        basket_tokens,
        asset_tokens,
        asset_prices,
        target_weights,
        penalty_params,
        main=True,
    ):
        self.basket_tokens = int(basket_tokens)
        self.asset_tokens = np.array(asset_tokens, dtype=np.int64)

        self.asset_prices = np.array(asset_prices, dtype=np.longdouble)
        self.target_weights = np.array(target_weights, dtype=np.int64)

        self.penalty_params = penalty_params

        self.main = main

    @classmethod
    def from_interface(cls, interface):
        return cls(
            interface.basket_tokens,
            interface.asset_tokens,
            interface.asset_prices,
            interface.target_weights,
            interface.penalty_params,
            False,
        )

    def penalty(self, x):
        if x <= 0:
            return 1 - self.penalty_params["a_neg"] * math.tanh(
                x / self.penalty_params["s_neg"]
            )
        else:
            return 1 - self.penalty_params["a_pos"] * math.tanh(
                x / self.penalty_params["s_pos"]
            )

    def mint(self, amounts, min_tokens=None):

        amounts = np.array(amounts, dtype=np.int64)
        if not np.any(amounts):
            return 0

        i = self.asset_tokens
        w = self.target_weights
        c = amounts
        p = self.asset_prices
        n = self.basket_tokens

        u = w * p / np.dot(w, p)

        def err(i, p):
            return u * np.dot(i, p) - i * p

        d = np.abs(err(i + c, p)) - np.abs(err(i, p))
        x = np.sum(d) / np.dot(c, p) / 2
        m = int(self.penalty(x) * n * np.dot(c, p) / np.dot(i, p))

        if min_tokens is not None and m < min_tokens:
            raise MinimumNotMetException(
                f"Basked would mint {m}, but {min_tokens} requested"
            )

        self.asset_tokens = i + c
        self.basket_tokens += m

        if DEBUG and self.main:
            print("MINTING", amounts, m)

        return m

    def redeem(self, amount, weights=None, min_tokens=None):

        amount = int(amount)

        if not amount:
            return np.zeros(shape=(len(self.asset_tokens),), dtype=np.int64)

        r = weights
        i = self.asset_tokens
        w = self.target_weights
        m = amount
        p = self.asset_prices
        n = self.basket_tokens

        if r is None:
            r = i

        r = r / np.sum(r)

        u = w * p / np.dot(w, p)
        b = r * m / n * np.dot(i, p) / np.dot(r, p)

        def err(i, p):
            return u * np.dot(i, p) - i * p

        d = np.abs(err(i - b, p)) - np.abs(err(i, p))
        x = np.sum(d) / np.dot(b, p) / 2
        r = b * self.penalty(x)

        r = r.astype(np.int64)

        if min_tokens is not None and (min_tokens > r).any():
            raise MinimumNotMetException(
                f"Basket would redeem {r}, but {min_tokens} requested"
            )

        self.basket_tokens -= m
        assert self.basket_tokens >= 0

        self.asset_tokens -= r

        assert (self.asset_tokens >= 0).all()

        if DEBUG and self.main:
            print("REDEEMING", amount, weights, r)

        return r

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
