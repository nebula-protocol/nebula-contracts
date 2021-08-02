import os
import asyncio
import yfinance as yf

os.environ["MNEMONIC"] = mnemonic = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'
os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, chain, ClusterContract, terra, deployer
from easy_mint import EasyMintOptimizer
from sim_utils import unpack_asset_info, get_pair_contract_uusd
import time
import sys

INTERVAL = 5 * 60

class RebalancingBot:
    def __init__(self, factory_contract, incentives_contract, gain_threshold=2000000):
        self.factory_contract = factory_contract

        # All mints and redeems will be eligible for rewards
        self.incentives_contract = incentives_contract

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
        Actually does minting and redeeming
        """
        cluster = self.clusters[idx]
        optimizer = self.easy_mint_optimizers[idx]

        target_assets = optimizer.cluster_simulator.target_assets
        pair_contracts = [ts_sim.pair_contract for ts_sim in optimizer.terraswap_simulators]

        # Might have to split into two transactions if gas fees are an issue
        msgs = []

        # Swap from UST on Terraswap pair
        for idx, p in enumerate(pair_contracts):
            amt = str(uusd_distrs[idx])
            msgs.append(
                p.swap(
                    offer_asset=Asset.asset("uusd", amount=amt, native=True),
                    _send={"uusd": amt},
                )
            )

        res_swaps = chain(*msgs)
        ret_asset_vals = [int(log.events_by_type['from_contract']['return_amount'][0]) for log in res_swaps.logs]

        # Prepare to incentives mint on cluster
        msgs = []
        mint_assets = []
        send = {}
        for idx, t in enumerate(target_assets):
            asset_name, is_native = unpack_asset_info(t)
            amt_to_mint = str(ret_asset_vals[idx])
            send = None
            # Prepare for sending
            if is_native:
                send[asset_name] = amt_to_mint
            else:
                # Must increase allowance before incentives minting
                token_contract = Contract(asset_name)
                token_contract.increase_allowance(amount=amt_to_mint, spender=self.incentives_contract)
            
            mint_assets.append(Asset.asset(asset_name, amt_to_mint, native=is_native))

        # Finally, incentives mint on cluster
        msgs.append(
            self.incentives.mint(
                cluster_contract=cluster,
                asset_amounts=mint_assets,
                _send=send
            )
        )

        res_mint = await chain(*msgs)
        ct_tokens_received = int(res_mint.logs[-1].events_by_type["from_contract"]["mint_to_sender"][0])

        # Increase cluster token allowance
        cluster_token = optimizer.cluster_simulator.cluster_state['cluster_token']
        res_redeem = await chain(
            cluster_token.increase_allowance(spender=self.incentives_contract, amount=str(ct_tokens_received)),
            self.incentives.redeem(
                max_tokens=str(ct_tokens_received),
                cluster_contract=cluster,
            ),
        )

        # Incentives pro-rata redeem on cluster [TODO: Smart redeem]

        # This is a string like '[1, 1]' so we must convert
        redeem_amts = res_redeem.logs[-1].events_by_type["from_contract"]['redeem_totals'][0]
        redeem_asset_vals = [int(r) for r in redeem_amts[1:-1].split(',')]

        # Swap back from tokens / native tokens to UST
        msgs = []
        for idx, t in enumerate(target_assets):
            p = pair_contracts[idx]
            asset_name, is_native = unpack_asset_info(t)
            amt_to_mint = str(redeem_asset_vals[idx])
            send = None

            # Prepare for sending
            if is_native:
                send = {asset_name: amt_to_mint}
            else:
                # Must increase allowance before swapping cw20
                token_contract = Contract(asset_name)
                msgs.append(token_contract.increase_allowance(amount=amt_to_mint, spender=p))
            msgs.append(
                p.swap(
                    offer_asset=Asset.asset(asset_name, amount=amt_to_mint, native=is_native),
                    _send=send,
                )
            )
        await chain(*msgs)

        uusd_ret_asset_vals = [int(log.events_by_type['from_contract']['return_amount'][0]) for log in res_swaps.logs]

        print(f"Rebalancing net us {uusd_ret_asset_vals - sum(uusd_distrs)}")

        return sum(uusd_ret_asset_vals)


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

async def run_rebalance_periodically(factory_contract, incentives_contract, interval):
    rebalancing_bot = RebalancingBot(factory_contract, incentives_contract)

    while True:
        await rebalancing_bot.reset_infos()
        print(rebalancing_bot.clusters)
        print(rebalancing_bot.uusd_balance)
        await asyncio.gather(
            asyncio.sleep(interval),
            RebalancingBot.perform_rebalance(),
        )

# Playground function to look at logs
async def testing():
    uluna_pair = await get_pair_contract_uusd(Asset.asset('uluna', 0, native=True))
    anc_pair = await get_pair_contract_uusd(Asset.asset('terra1747mad58h0w4y589y3sk84r5efqdev9q4r02pc', 0, native=False))

    pair_contracts = [Contract(uluna_pair), Contract(anc_pair)]

    uusd_distrs = [10000, 10000]

    msgs = []
    for idx, p in enumerate(pair_contracts):
        amt = str(uusd_distrs[idx])
        msgs.append(
            p.swap(
                offer_asset=Asset.asset("uusd", amount=amt, native=True),
                _send={"uusd": amt},
            )
        )

    result = await chain(*msgs)
    print(result)

    assert(len(pair_contracts) == len(result.logs))
    print('logs be like', result.logs[0])
    res_logs = result.logs[1]

    for log in result.logs:
        ret_val = int(log.events_by_type['from_contract']['return_amount'][0])
        print(ret_val)
    
    import pdb; pdb.set_trace()
    
    print('goddamn bro')

if __name__ == "__main__":
    # asyncio.get_event_loop().run_until_complete(testing())
    interval = INTERVAL
    factory_contract = Contract(sys.argv[1])
    incentives_contract = Contract(sys.argv[2])
    asyncio.get_event_loop().run_until_complete(run_rebalance_periodically(factory_contract, incentives_contract, interval))