import os
import asyncio
import yfinance as yf

os.environ["MNEMONIC"] = mnemonic = 'soda buffalo melt legal zebra claw taxi peace fashion service drastic special coach state rare harsh business bulb tissue illness juice steel screen chef'
os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, ClusterContract, terra, deployer
from easy_mint import EasyMintOptimizer
from sim_utils import unpack_asset_info
import time
import sys

INTERVAL = 5 * 60

factory_contract = Contract(sys.argv[1])

class RebalancingBot:
    def __init__(self, factory_contract, gain_threshold=2000000):
        self.factory_contract = factory_contract

        # How much to gain via rebalancing to do the trade
        self.gain_threshold = gain_threshold

    async def set_initial_state(self):
        contract_infos = await self.factory_contract.query.cluster_list()
        self.clusters = [Contract(c[0]) for c in contract_infos if c[1]]
        self.uusd_balance = await terra.bank.balance(address=deployer.key.acc_address)
        self.easy_mint_optimizers = [EasyMintOptimizer(c) for c in self.cluster]

    async def reset_infos(self):
        contract_infos = await self.factory_contract.query.cluster_list()
        self.clusters = [Contract(c[0]) for c in contract_infos if c[1]]
        self.uusd_balance = await terra.bank.balance(address=deployer.key.acc_address)
        for e in self.easy_mint_optimizers:
            e.reset_initial_state()


    async def simulate_full_loop(self, idx, uusd_cost, initial_reset=True):
        """
        Simulates full loop for specific cluster and returns expected reward
        """
        cluster = self.clusters[idx]
        optimizer = self.easy_mint_optimizers[idx]
        optimal_asset_allocation, uusd_per_asset, expected_cluster_tokens = optimizer.find_optimal_allocation(uusd_cost)

        if initial_reset:
            # Hard reset using blockchain
            optimizer.reset_initial_state()

        cluster_simulator = optimizer.cluster_simulator
        terraswap_simulators = optimizer.terraswap_simulators 

        asset_allocs = []

        # Executing Terraswap swaps in simulation (stateful change)
        for idx, sp in enumerate(uusd_per_asset):
            offer_asset = Asset.asset('uusd', sp, native=True)
            ret, spr, comm = terraswap_simulators[idx].execute_swap(offer_asset)
            asset_allocs.append(ret)

        # Execute mint in simulator (stateful change)
        cluster_token_amt = cluster_simulator.execute_mint(asset_allocs)

        # Redeem pro-rata
        redeem_cost, returned_amts = cluster_simulator.execute_redeem(cluster_tokens=cluster_token_amt)

        assert redeem_cost == cluster_token_amt

        uusd_returned = 0

        # Convert returned_amt back to UST by executing swaps in simulation
        for idx, asset_amt in enumerate(returned_amts):
            asset_name, is_native = unpack_asset_info(cluster_simulator.target_assets[idx])
            offer_asset = Asset.asset(asset_name, asset_amt, native=is_native)
            ret, spr, comm = terraswap_simulators[idx].execute_swap(offer_asset)
            uusd_returned += ret

        gain = uusd_returned - uusd_cost

        # Return if we should do this transaction and how much per asset we want
        return gain > self.gain_threshold, uusd_per_asset

    async def execute_easy_mint_and_redeem(self, idx, uusd_distrs):
        """
        Simulates full loop for specific cluster and returns expected reward
        """
        cluster = self.clusters[idx]
        optimizer = self.easy_mint_optimizers[idx]
        optimal_asset_allocation, uusd_per_asset, expected_cluster_tokens = optimizer.find_optimal_allocation(uusd_cost)

        target_assets = optimizer.cluster_simulator.target_assets
        pair_contracts = [ts_sim.pair_contract for ts_sim in optimizer.terraswap_simulators]

        # Swap from UST on Terraswap pair

        # Mint or incentives mint on cluster 

        # Increase allowance / craft send message for native assets

        # Incentives mint on cluster 

        # Increase cluster token allowance

        # Incentives pro-rata redeem on cluster [TODO: Smart redeem]

        # Increase allowance again on tokens / craft send message

        # Swap back from tokens / native tokens to UST


    async def perform_rebalance(self):
        # Check how much balance I have
        self.uusd_balance = await terra.bank.balance(address=deployer.key.acc_address)

        # Might have to get block height here
        self.reset_infos()

        # Can maybe make a list and do weighted average by notional imbalance later
        uusd_chunks = [self.uusd_balance / len(self.clusters) for _ in self.cluster]

        for i in range(len(self.clusters)):
            uusd_chunk = uusd_chunks[i]
            should_rebalance, uusd_distribution = self.simulate_full_loop(i, uusd_chunk, initial_reset=True)
            if should_rebalance:
                print("Found rebalance opportunity")
                await self.execute_easy_mint_and_redeem(i, uusd_distribution)

async def run_rebalance_periodically(cluster_contract, interval):
    start_time = time.time()
    
    rebalancing_bot = RebalancingBot(factory_contract)

    while True:
        await rebalancing_bot.set_infos()
        print(rebalancing_bot.clusters)
        print(rebalancing_bot.uusd_balance)
        await asyncio.gather(
            asyncio.sleep(interval),
            RebalancingBot.perform_rebalance(),
        )

if __name__ == "__main__":
    interval = INTERVAL
    asyncio.get_event_loop().run_until_complete(run_rebalance_periodically(factory_contract, interval))