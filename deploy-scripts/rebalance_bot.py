import os
import asyncio
import yfinance as yf

os.environ["MNEMONIC"] = mnemonic = 'soda buffalo melt legal zebra claw taxi peace fashion service drastic special coach state rare harsh business bulb tissue illness juice steel screen chef'
os.environ["USE_TEQUILA"] = "1"

from api import Asset
from contract_helpers import Contract, ClusterContract, terra, deployer
import time
import sys

INTERVAL = 5 * 60

factory_contract = Contract(sys.argv[1])

class RebalancingBot:
    def __init__(self, factory_contract):
        self.factory_contract = factory_contract

    async def reset_infos(self):
        contract_infos = await self.factory_contract.query.cluster_list()
        self.clusters = [Contract(c[0]) for c in contract_infos if c[1]]
        self.uusd_balance = await terra.bank.balance(address=deployer.key.acc_address)

    # async def 

    async def perform_rebalance(self):
        return 

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