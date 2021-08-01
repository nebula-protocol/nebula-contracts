import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from contract_helpers import Contract
from base import terra
from sim_utils import *
import asyncio
from api import Asset
import sys
import numpy as np

pair_contract = Contract(sys.argv[1])

class TerraswapPoolSimulation:
    def __init__(self, pair_contract, commission_rate=0.03):
        """
        Takes in a pair contract. Use {pair} query to find this contract
        for a given token.
        """
        self.pair_contract = pair_contract
        self.commission_rate = commission_rate

    async def reset(self):
        """
        Reset to current state of the pool -- call once before simulation
        """
        self.pool_info = await self.pair_contract.query.pool()
        self.asset0 = unpack_asset(self.pool_info['assets'][0])
        self.asset1 = unpack_asset(self.pool_info['assets'][1])

    def compute_swap(self, offer_pool_amt, ask_pool_amt, offer_amount):
        """
        Offer -> ask
        """
        cp = offer_pool_amt * ask_pool_amt
        return_amt = (ask_pool_amt - cp / (offer_pool_amt + offer_amount))
        spread_amt = offer_amount * ask_pool_amt / offer_pool_amt - return_amt
        commission_amt = return_amt * self.commission_rate
        return_amt -= commission_amt
        return int(return_amt), int(spread_amt), int(commission_amt)

    def compute_offer_amt(self, offer_pool_amt, ask_pool_amt, ask_amount):
        """
        Ask -> offer
        """
        cp = offer_pool_amt * ask_pool_amt
        one_minus_commission = 1 - self.commission_rate

        offer_amt = cp / (ask_pool_amt - ask_amount * 1 / one_minus_commission) - offer_pool_amt
        before_commission_deduction = ask_amount * 1 / one_minus_commission
        spread_amt = (offer_amt * ask_pool_amt / offer_pool_amt) - before_commission_deduction
        commission_amt = before_commission_deduction * self.commission_rate

        return int(offer_amt), int(spread_amt), int(commission_amt)
        
    def simulate_swap(self, offer_asset: Asset=None, ask_asset: Asset=None):
        """
        Simulates a swap without changing internal state of the class.
        For most cases, offer_asset will be in UST
        """
        assert (offer_asset or ask_asset) and not (offer_asset and ask_asset)

        if offer_asset:
            offer_asset, native, offer_amount = unpack_asset(offer_asset)
            if offer_asset == self.asset0[0]:
                offer_pool_amt = self.asset0[2]
                ask_pool_amt = self.asset1[2]
            elif offer_asset == self.asset1[0]:
                offer_pool_amt = self.asset1[2]
                ask_pool_amt = self.asset0[2]
            else:
                raise NotImplementedError

            amt, spread_amt, commission_amt = self.compute_swap(offer_pool_amt, ask_pool_amt, offer_amount)
        elif ask_asset:
            ask_asset, native, ask_amount = unpack_asset(offer_asset)
            if offer_asset == self.asset0[0]:
                offer_pool_amt = self.asset0[2]
                ask_pool_amt = self.asset1[2]
            elif offer_asset == self.asset1[0]:
                offer_pool_amt = self.asset1[2]
                ask_pool_amt = self.asset0[2]
            else:
                raise NotImplementedError

            amt, spread_amt, commission_amt = self.compute_offer_amount(offer_pool_amt, ask_pool_amt, ask_amount)
        else:
            raise NotImplementedError

        return amt, spread_amt, commission_amt
        
    def execute_swap(self, offer_asset: Asset):
        """
        Changes internal state of the simulation.
        For most cases, offer_asset will be in UST
        """
        return_amt, spread_amt, commission_amt = self.simulate_swap(offer_asset=offer_asset)

        # Make stateful change
        offer_asset, native, offer_amount = unpack_asset(offer_asset)
        if offer_asset == self.asset0[0]:
            self.asset0[2] += offer_amount
            self.asset1[2] -= return_amt
        elif offer_asset == self.asset1[0]:
            self.asset1[2] += offer_amount
            self.asset0[2] -= return_amt
        else:
            raise NotImplementedError

        return return_amt, spread_amt, commission_amt

async def main():
    simulator = TerraswapPoolSimulation(pair_contract)
    await simulator.reset()
    print(simulator.pool_info)
    offer_asset = Asset.asset('uusd', 1000000 ,native=True)
    print(simulator.simulate_swap(offer_asset=offer_asset))


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
