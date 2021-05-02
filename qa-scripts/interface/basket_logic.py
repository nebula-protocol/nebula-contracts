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

        self.penalty_params = {k: float(v) for k, v in penalty_params.items()}

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

    def notional_penalty(self, i0, i1, w, p):
        def err(i, p, w):
            u = w * p / np.dot(w, p)
            return np.sum(np.abs(u * np.dot(i, p) - i * p))

        imb0 = err(i0, p, w)
        imb1 = err(i1, p, w)

        penalty_amt_lo = self.penalty_params["penalty_amt_lo"]
        penalty_cutoff_lo = self.penalty_params["penalty_cutoff_lo"]
        penalty_amt_hi = self.penalty_params["penalty_amt_hi"]
        penalty_cutoff_hi = self.penalty_params["penalty_cutoff_hi"]
        reward_amt = self.penalty_params["reward_amt"]
        reward_cutoff = self.penalty_params["reward_cutoff"]

        e = np.dot(i0, p)

        if imb0 > imb1:
            cutoff_lo = penalty_cutoff_lo * e
            cutoff_hi = penalty_cutoff_hi * e

            penalty_1 = (min(imb0, cutoff_lo) - min(imb1, cutoff_lo)) * penalty_amt_lo

            imb0_mid = min(max(imb1, cutoff_lo), cutoff_hi)
            imb1_mid = min(max(imb1, cutoff_lo), cutoff_hi)

            amt_gap = penalty_amt_hi - penalty_amt_lo
            cutoff_gap = cutoff_hi - cutoff_lo

            imb0_mid_height = (
                imb0_mid - cutoff_lo
            ) * amt_gap / cutoff_gap + penalty_amt_lo
            imb1_mid_height = (
                imb1_mid - cutoff_lo
            ) * amt_gap / cutoff_gap + penalty_amt_lo

            penalty_2 = (imb0_mid_height + imb1_mid_height) * (imb0_mid - imb1_mid) / 2

            penalty_3 = (max(imb1, cutoff_hi) - max(imb1, cutoff_hi)) * penalty_amt_hi
            return -(penalty_1 + penalty_2 + penalty_3)
        else:
            cutoff = reward_cutoff * e
            return (max(imb0, cutoff) - max(imb1, cutoff)) * reward_amt

    def mint(self, amounts, min_tokens=None):

        amounts = np.array(amounts, dtype=np.int64)
        if not np.any(amounts):
            return 0

        i0 = self.asset_tokens
        w = self.target_weights
        c = amounts
        p = self.asset_prices
        n = self.basket_tokens

        i1 = i0 + c

        m = int(
            n * (np.dot(c, p) + self.notional_penalty(i0, i1, w, p)) / np.dot(i0, p)
        )

        if min_tokens is not None and m < min_tokens:
            raise MinimumNotMetException(
                f"Basked would mint {m}, but {min_tokens} requested"
            )

        self.asset_tokens = i1
        self.basket_tokens += m
        return m

    def redeem(self, max_tokens, asset_amounts=None):

        r = asset_amounts
        i0 = self.asset_tokens
        w = self.target_weights
        m = max_tokens
        p = self.asset_prices
        n = self.basket_tokens

        if r is None:
            r = m * np.dot(i0, p) * w / n / np.dot(w, p)
            r = r.astype(np.int64)
        else:
            i1 = i0 - r
            cst = (
                n * (np.dot(r, p) - self.notional_penalty(i0, i1, w, p)) / np.dot(i0, p)
            )
            if cst > m:
                raise MinimumNotMetException(
                    f"Basket would cost {cst}, but {m} requested"
                )
            m = cst

        self.basket_tokens -= m
        assert self.basket_tokens >= 0

        self.asset_tokens -= r
        assert (self.asset_tokens >= 0).all()

        return m, r

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
