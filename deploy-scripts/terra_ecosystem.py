
import requests
import json
import os

from contract_helpers import Contract
import asyncio
from base import deployer


"""
Recomposes according to Total Value Locked (TVL) in the provided assets. 
WARN: Do not use with Mirrored Assets.
"""
class TerraFullDilutedMcapRecomposer:
    def __init__(self, asset_names, asset_tokens):
        self.asset_names = asset_names
        self.asset_tokens = asset_tokens
        os.environ["USE_TEQUILA"] = "1"
        self.asset_ids = [
            "terra-luna",
            "anchor-protocol",
            "mirror-protocol"
        ]
        self.api = "https://api.coingecko.com/api/v3/simple/price"
        self.currency = "usd"

    async def weighting(self):
        get_params = {
            "ids": ','.join(self.asset_ids),
            "vs_currencies": self.currency
        }
        r = requests.get(self.api, params=get_params)
        r.raise_for_status()
        prices = json.loads(r.text)
        asset_to_fdm = {}
        for i in range(len(self.asset_ids)):
            token_contract = self.asset_tokens[i]
            token_info = await token_contract.query.token_info()
            asset_id = self.asset_ids[i]
            price = float(prices[asset_id][self.currency])
            total_supply = float(token_info['total_supply'])
            fully_diluted_mcap = price * total_supply
            asset_to_fdm[asset_id] = fully_diluted_mcap
            print("{} has TVL of {}M".format(asset_id, fully_diluted_mcap/1000000.0))
        denom = sum(asset_to_fdm.values())
        print("Total FDM of all assets: {}M".format(denom/1000000.0))
        target = [asset_to_fdm[asset]/denom for asset in asset_to_fdm]
        return target
        
    
    async def recompose(self):
        target_weights = await self.weighting()
        print(self.asset_names, target_weights)
        target_weights = [int(100 * target_weight) for target_weight in target_weights]
        return self.asset_names, target_weights

async def main():
    asset_names = ['LUNA', 'ANC', 'MIR']
    rec_bot = TerraFullDilutedMcapRecomposer(asset_names)
    await rec_bot.recompose()

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
