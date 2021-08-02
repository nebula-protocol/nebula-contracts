import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from contract_helpers import Contract
import asyncio
import sys
import numpy as np

cluster = Contract(sys.argv[1])

class ClusterSimulatorWithPenalty:
    def __init__(self, cluster_contract, collector_fee = 0.001):
        self.cluster_contract = cluster_contract
        self.collector_fee = collector_fee
        
    async def reset_initial_state(self):
        cluster_state = await cluster.query.cluster_state(cluster_contract_address=self.cluster_contract)
        self.cluster_state = cluster_state

        self.penalty = Contract(cluster_state['penalty'])
        self.supply = float(cluster_state['outstanding_balance_tokens'])

        self.penalty_info = await self.penalty.query.params()
        curr_ema = float(self.penalty_info['ema'])
        last_block = int(self.penalty_info['last_block'])
        params = {k: float(v) for k,v in self.penalty_info['penalty_params'].items()}

        self.curr_ema, self.penalty_params, self.last_block = curr_ema, params, last_block

        self.base_inv = np.array([float(i) for i in cluster_state['inv']])
        self.prices = np.array([float(p) for p in cluster_state['prices']])

        self.target_assets = [t['info'] for t in cluster_state['target']]
        self.target_amounts = np.array([int(t['amount']) for t in cluster_state['target']])

    def imbalance(self, inv):
        """
        Expecting inv to be np array
        """
        wp = np.dot(self.target_amounts, self.prices)
        u = self.target_amounts * self.prices

        ip = np.dot(inv, self.prices)
        v = inv * self.prices

        err_portfolio = u * ip - wp * v
        return np.sum(np.abs(err_portfolio)) / wp

    def get_ema(self, block_height, net_asset_value):
        if self.last_block != 0:
            dt = block_height - self.last_block
            tau = -600
            factor = np.exp(dt/tau)
            return factor * self.curr_ema + (1-factor) * net_asset_value
        else:
            return net_asset_value

    def update_ema(self, block_height, net_asset_value):
        self.curr_ema = self.get_ema(block_height, net_asset_value)
    

    def notional_penalty(self, block_height, curr_inv, new_inv):
        imb0 = self.imbalance(curr_inv)
        imb1 = self.imbalance(new_inv)

        nav = np.dot(curr_inv, self.prices)
        e = min(self.get_ema(block_height, nav), nav)

        penalty_amt_lo = self.penalty_params['penalty_amt_lo']
        penalty_cutoff_lo = self.penalty_params['penalty_cutoff_lo']
        penalty_amt_hi = self.penalty_params['penalty_amt_hi']
        penalty_cutoff_hi = self.penalty_params['penalty_cutoff_hi']
        reward_amt = self.penalty_params['reward_amt']
        reward_cutoff = self.penalty_params['reward_cutoff']

        if imb0 < imb1:
            cutoff_lo = penalty_cutoff_lo * e
            cutoff_hi = penalty_cutoff_hi * e

            if imb1 > cutoff_hi:
                print("this move causes cluster imbalance too high error but we will ignore")

            # penalty function is broken into three pieces, where its flat, linear, and then flat
            # compute the area under each piece separately

            penalty_1 = (min(imb1, cutoff_lo) - min(imb0, cutoff_lo)) * penalty_amt_lo

            # clip to only middle portion
            imb0_mid = min(max(imb0, cutoff_lo), cutoff_hi)
            imb1_mid = min(max(imb1, cutoff_lo), cutoff_hi)

            amt_gap = penalty_amt_hi - penalty_amt_lo
            cutoff_gap = cutoff_hi - cutoff_lo

            # value of y when x is at imb0_mid and imb1_mid respectively
            imb0_mid_height = (imb0_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo
            imb1_mid_height = (imb1_mid - cutoff_lo) * amt_gap / cutoff_gap + penalty_amt_lo

            # area of a trapezoid
            penalty_2 = (imb0_mid_height + imb1_mid_height) * (imb1_mid - imb0_mid) / 2
            penalty_3 = (max(imb1, cutoff_hi) - max(imb0, cutoff_hi)) * penalty_amt_hi

            return -(penalty_1 + penalty_2 + penalty_3)
        else:
            # use reward function
            cutoff = reward_cutoff * e
            return (max(imb0, cutoff) - max(imb1, cutoff)) * reward_amt

    def get_notional_value_of_ct(self):
        return np.dot(self.base_inv, self.prices) / self.supply

    def simulate_mint(self, amts, block_height=None, inv=None):
        """
        amts: list of token counts to add to self.base_inv
        """
        if inv is None:
            inv = self.base_inv

        if block_height is None:
            block_height = self.last_block

        amts = np.array(amts)

        penalty = self.notional_penalty(block_height, inv, inv + amts)

        notional_value = np.dot(amts, self.prices) + penalty
        mint_subtotal = self.supply * notional_value / np.dot(inv, self.prices)

        return mint_subtotal

    def simulate_redeem(self, amts=None, cluster_tokens = None, block_height=None, inv=None):
        """
        amts: list of token counts to add to self.base_inv

        NOTE: This returns how much cluster token it will cost to redeem amts if
        amts is not empty
        """
        if inv is None:
            inv = np.array(self.base_inv)

        if block_height is None:
            block_height = self.last_block

        assert (cluster_tokens or amts)

        if amts is not None:
            amts = np.array(amts)

            penalty = self.notional_penalty(block_height, inv, inv + amts)
            notional_value = np.dot(amts, self.prices) - penalty
            redeem_cost = self.supply * notional_value / np.dot(inv, self.prices)
            return redeem_cost, amts

        if cluster_tokens is not None:
            redeem_arr =  inv * cluster_tokens / self.supply
            return cluster_tokens, redeem_arr

        raise NotImplementedError
        

    def smart_redeem(self, cluster_tokens_chunk, idx, block_height=None, inv=None):
        """
        Description: We want to find out the amount of asset[idx] to redeem against 
        that will cost cluster_tokens_chunk. The current method is a binary search.
        """
        if inv is None:
            inv = np.array(self.base_inv)

        if block_height is None:
            block_height = self.last_block

        amt_low = 0
        amt_high = 10 # Use some heuristic too like 2 * notional value 

        return idx

    def execute_mint(self, amts, block_height=None):
        if block_height is None:
            block_height = self.last_block
        mint_amt = self.simulate_mint(amts, block_height=block_height, inv=None)
        self.supply += mint_amt
        self.base_inv += amts
        self.update_ema(block_height, np.dot(self.base_inv, self.prices))
        return mint_amt

    def execute_redeem(self, amts=None, cluster_tokens = None, block_height=None):
        if block_height is None:
            block_height = self.last_block
        redeem_cost, returned_amts = self.simulate_redeem(amts=amts, cluster_tokens=cluster_tokens, block_height=block_height)
        # Burn
        self.supply -= redeem_cost
        self.base_inv -= amts
        self.update_ema(block_height, np.dot(self.base_inv, self.prices))
        return redeem_cost, returned_amts

    def reset_to_cluster_state(self):
        self.base_inv = np.array([float(i) for i in self.cluster_state['inv']])
        self.supply = float(self.cluster_state['outstanding_balance_tokens'])
        self.last_block = int(self.penalty_info['last_block'])
        self.curr_ema = float(self.penalty_info['ema'])

    def get_curr_imbalance(self):
        return self.imbalance(self.base_inv)

async def main():
    simulator = ClusterSimulatorWithPenalty(cluster)
    await simulator.reset_initial_state()
    print(simulator.cluster_state)
    print(simulator.simulate_mint([10, 10, 10]))
    print(np.dot(simulator.prices, [10, 10, 10]))
    print(simulator.get_notional_value_of_ct())


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
