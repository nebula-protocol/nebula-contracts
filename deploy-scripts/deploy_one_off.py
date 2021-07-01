import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = "insert mnemonic here"

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract
import asyncio
from base import deployer

REQUIRE_GOV = True


async def deploy_contracts():

    print('Initializing all base contracts and such onto Tequila')
    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)
    import pdb; pdb.set_trace()
    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    await ecosystem.initialize_base_contracts()
    await ecosystem.initialize_extraneous_contracts()

    print(ecosystem.__dict__)



if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_contracts())
