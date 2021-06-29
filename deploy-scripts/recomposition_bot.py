from bot_code.bullish_cross import BullishCrossRecomposer
from api import Asset

class RecompositionBot:
    def __init__(self, bot_name, asset_names, ecosystem):
        self.ecosystem = ecosystem
        if bot_name == 'bullish-cross':
            self.recomposer = BullishCrossRecomposer(asset_names, use_test_data=True)
        else:
            raise NotImplementedError


    async def recompose(self, asset_names):
        new_assets, new_target = await self.recomposer.recompose()

        await self.ecosystem.cluster.reset_target(
            assets=[Asset.cw20_asset_info(a) for a in new_assets],
            target=new_target
        )

        return new_assets, new_target