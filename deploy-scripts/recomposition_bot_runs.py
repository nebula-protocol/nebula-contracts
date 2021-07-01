from ecosystem import Ecosystem
import asyncio
from recomposition_bot import RecompositionBot
from bot_code.bullish_cross import BullishCrossRecomposer
from api import Asset

import time

async def main():

    ecosystem = Ecosystem(require_gov=True)
    await ecosystem.initialize_base_contracts()
    await ecosystem.initialize_extraneous_contracts()
    bot = 'terra-ecosystem'
    if bot == 'tvl':
        tvl_assets = ["AAVE", "COMP", "MKR", "CREAM", "ANC"]
        await ecosystem.create_cluster(
            100,
            [100, 100, 100, 100, 100],
            [100, 100, 100, 100, 100],
            [1, 1, 1, 1, 1],
            {
                "penalty_amt_lo": "0.1",
                "penalty_cutoff_lo": "0.01",
                "penalty_amt_hi": "0.5",
                "penalty_cutoff_hi": "0.1",
                "reward_amt": "0.05",
                "reward_cutoff": "0.02",
            },
            asset_names=tvl_assets
        )
        rec_bot = RecompositionBot('tvl-locked', tvl_assets, ecosystem)
    elif bot == 'bullish-cross':
        asset_names = ['mFB', 'mTSLA', 'mGOOGL']
        asset_addresses = [
            'terra1mqsjugsugfprn3cvgxsrr8akkvdxv2pzc74us7', 
            'terra14y5affaarufk3uscy2vr6pe6w6zqf2wpjzn5sh', 
            'terra1h8arz2k547uvmpxctuwush3jzc8fun4s96qgwt'
        ]
        await ecosystem.create_cluster(
            100,
            [100, 100, 100],
            [100, 100, 100],
            [1, 1, 1],
            {
                "penalty_amt_lo": "0.1",
                "penalty_cutoff_lo": "0.01",
                "penalty_amt_hi": "0.5",
                "penalty_cutoff_hi": "0.1",
                "reward_amt": "0.05",
                "reward_cutoff": "0.02",
            },
            asset_names=asset_names
        )
        rec_bot = RecompositionBot('bullish-cross', asset_addresses, ecosystem)
    elif bot == 'terra-ecosystem':
        asset_names = ['LUNA', 'ANC', 'MIR']
        await ecosystem.create_cluster(
            100,
            [100, 100, 100],
            [100, 100, 100],
            [1, 1, 1],
            {
                "penalty_amt_lo": "0.1",
                "penalty_cutoff_lo": "0.01",
                "penalty_amt_hi": "0.5",
                "penalty_cutoff_hi": "0.1",
                "reward_amt": "0.05",
                "reward_cutoff": "0.02",
            },
            asset_names=asset_names
        )
        rec_bot = RecompositionBot('terra-ecosystem', asset_names, ecosystem)
    else:
        raise NotImplementedError
    
    await rec_bot.run_recomposition_periodically(5)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
