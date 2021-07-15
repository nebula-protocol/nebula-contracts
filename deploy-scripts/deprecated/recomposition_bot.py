from bot_code.bullish_cross import BullishCrossRecomposer
from bot_code.tvl_locked import TVLLockedRecomposer
from bot_code.momentum_trading import MomentumTradingRecomposer
from terra_ecosystem import TerraFullDilutedMcapRecomposer
from api import Asset

import time
import asyncio

class RecompositionBot:
    def __init__(self, bot_name, assets, ecosystem):
        self.ecosystem = ecosystem
        if bot_name == 'bullish-cross':
            self.recomposer = BullishCrossRecomposer(assets)
        elif bot_name == 'tvl-locked':
            self.recomposer = TVLLockedRecomposer(assets)
        elif bot_name == 'terra-ecosystem':
            self.recomposer = TerraFullDilutedMcapRecomposer(assets, ecosystem.asset_tokens)
        elif bot_name == 'momentum_trading':
            self.recomposer = MomentumTradingRecomposer()
        else:
            raise NotImplementedError


    async def recompose(self):
        new_assets, new_target = await self.recomposer.recompose()

        await self.ecosystem.cluster.reset_target(
            assets=[Asset.cw20_asset_info(a) for a in new_assets],
            target=new_target
        )

        return new_assets, new_target
    
    async def run_recomposition_periodically(self, interval):
        start_time = time.time()

        while True:
            print(round(time.time() - start_time, 1), "Recomposition")
            await asyncio.gather(
                asyncio.sleep(interval),
                self.recompose(),
            )