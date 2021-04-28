import random
from ..strategy_base import StrategyBase


class HypeFlow(StrategyBase):
    def __init__(self, basket, threshold=0.95):

        super().__init__(basket, [], 0, graph=False)
        self.threshold = threshold
        self.basket = basket
        self.inv = None

        self.pos_hype = random.randrange(0, len(self.basket.asset_tokens))
        self.neg_hype = random.randrange(0, len(self.basket.asset_tokens))
        while self.neg_hype == self.pos_hype:
            self.neg_hype = random.randrange(0, len(self.basket.asset_tokens))

    def tick(self):
        # - favor minting with one type of token
        # - favor redeeming with one type of token
        # - swap out hyped up tokens ever so often
        if random.random() < 0.005:
            self.pos_hype = random.randrange(0, len(self.basket.asset_tokens))
            self.neg_hype = random.randrange(0, len(self.basket.asset_tokens))
            while self.neg_hype == self.pos_hype:
                self.neg_hype = random.randrange(0, len(self.basket.asset_tokens))
        while True:
            if random.random() < 0.5:
                redeem_amt = random.randint(1, 20) * self.basket_tokens // 100
                if self.basket.basket_tokens - 100 >= redeem_amt and redeem_amt > 0:
                    redeem_weights = [0] * len(self.basket.asset_tokens)
                    redeem_weights[self.pos_hype] = 1
                    self.redeem(redeem_amt, weights=redeem_weights)

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
                amt = sim.mint(deposit_amt)

                if amt * sim.token_value > self.threshold * deposit_val:
                    deposit_amt[self.neg_hype] = max(deposit_amt)
                    self.mint(deposit_amt)
                    break
                else:
                    continue
