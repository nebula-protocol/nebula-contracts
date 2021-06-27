class RecompositionBot:
    def __init__(self, bot_name):
        if bot_name == 'bullish-cross':
            self.recomposer = bullish_cross_recomposer
        else:
            raise NotImplementedError


    # background contracts needed to create cluster contracts
    async def recompose(self, asset_names):
        return self.recomposer.recompose(asset_names, testing=True)