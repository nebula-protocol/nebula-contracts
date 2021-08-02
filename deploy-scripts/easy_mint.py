import os
from typing import List

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from contract_helpers import Contract
from sim_utils import *
from simulation_cluster_ops import ClusterSimulatorWithPenalty
from simulation_terraswap import TerraswapPoolSimulation
import asyncio
from api import Asset
import sys
import numpy as np

cluster_contract = Contract(sys.argv[1])
uusd_amount = int(sys.argv[2])

def find_optimal_allocation(cluster_sim: ClusterSimulatorWithPenalty, ts_sims: List[TerraswapPoolSimulation], uusd_amt):
    order_uusd = int(np.log10(uusd_amt)) - 6
    num_chunks = max(pow(10, order_uusd), 100)
    uusd_per_chunk = uusd_amt / num_chunks

    optimal_asset_allocation = [0 for i in range(len(cluster_sim.base_inv))]
    uusd_per_asset = [0 for i in range(len(cluster_sim.base_inv))]
    expected_cluster_tokens = 0
    
    print("Iterating through", num_chunks)

    for i in range(num_chunks):
        # Greedily pick the asset that maximizes number of minted cluster tokens
        max_asset_index, max_cluster_amt, best_asset_amt = -1, 0, 0
        for j in range(len(cluster_sim.target_assets)):
            asset = cluster_sim.target_assets[j]
            ts_sim = ts_sims[j]

            offer_asset = Asset.asset('uusd', uusd_per_chunk, native=True)

            asset_amt, _, _ = ts_sim.simulate_swap(offer_asset=offer_asset)

            add_amts = [0 for _ in range(len(cluster_sim.target_assets))]
            add_amts[j] = asset_amt

            cluster_token_amt = cluster_sim.simulate_mint(add_amts)

            if cluster_token_amt > max_cluster_amt:
                max_cluster_amt = cluster_token_amt
                max_asset_index = j
                best_asset_amt = asset_amt

        assert max_asset_index != -1

        # Make choice here
        optimal_asset_allocation[max_asset_index] += best_asset_amt
        uusd_per_asset[max_asset_index] += uusd_per_chunk

        expected_cluster_tokens += max_cluster_amt

        # Update terraswap sim with trade
        offer_asset = Asset.asset('uusd', uusd_per_chunk, native=True)
        ts_sims[max_asset_index].execute_swap(offer_asset=offer_asset)

        # Update cluster sim with mint
        add_amts = [0 for _ in range(len(cluster_sim.target_assets))]
        add_amts[max_asset_index] = best_asset_amt
        cluster_sim.execute_mint(add_amts)

    return optimal_asset_allocation, uusd_per_asset, expected_cluster_tokens


async def main():
    cluster_simulator = ClusterSimulatorWithPenalty(cluster_contract)
    await cluster_simulator.set_initial_state()
    target = cluster_simulator.cluster_state['target']
    print("Current state", cluster_simulator.cluster_state)
    prices = [float(p) for p in cluster_simulator.cluster_state['prices']]
    pair_contracts = [Contract(await get_pair_contract_uusd(t, price=prices[idx])) for idx, t in enumerate(target)]
    print(pair_contracts)

    terraswap_simulators = [TerraswapPoolSimulation(p) for p in pair_contracts]
    for ts in terraswap_simulators:
        await ts.reset()


    print("Base inv before algo", cluster_simulator.base_inv)
    print("Imbalance before algo", cluster_simulator.get_curr_imbalance())

    opt_allocs, opt_uusd_spends, expected_ct_amount = find_optimal_allocation(cluster_simulator, terraswap_simulators, uusd_amount)
    print(f'Opt asset alloc: {opt_allocs}, opt uusd spend: {opt_uusd_spends}, expected amount: {expected_ct_amount}')

    print("Base inv after algo", cluster_simulator.base_inv)
    print("Imbalance after algo", cluster_simulator.get_curr_imbalance())


    print("SANITY CHECK")

    # Sanity check
    for ts in terraswap_simulators:
        await ts.reset()

    cluster_simulator.reset_to_cluster_state()
    print("Imbalance before algo", cluster_simulator.get_curr_imbalance())
    print("Base inv before algo", cluster_simulator.base_inv)
    asset_allocs = []
    for idx, sp in enumerate(opt_uusd_spends):
        print(f"Spending {sp} uusd on index {idx}")
        offer_asset = Asset.asset('uusd', sp, native=True)
        ret, spr, comm = terraswap_simulators[idx].execute_swap(offer_asset)
        asset_allocs.append(ret)

    print(f"Actual asset alloc {asset_allocs}")


    cluster_token_amt = cluster_simulator.execute_mint(asset_allocs)

    print(f"Actual cluster token amt {cluster_token_amt}")
    print("Imbalance after algo", cluster_simulator.get_curr_imbalance())
    print("Base inv after algo", cluster_simulator.base_inv)

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())

