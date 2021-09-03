from api import Asset
import os
os.environ["USE_BOMBAY"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from ecosystem_deploy import Ecosystem

from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer
import pprint

REQUIRE_GOV = True


async def deploy_contracts():
    print('Initializing all base contracts and such onto Tequila')
    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)
    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    await ecosystem.initialize_contracts()

    pprint.pprint(ecosystem.__dict__)



if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_contracts())
