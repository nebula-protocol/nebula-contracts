import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio
from base import deployer
from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV

REQUIRE_GOV = True


async def deploy_terra_ecosystem():
    cluster = Contract(
        "terra1ae2amnd99wppjyumwz6qet7sjx6ynq39g8zha5"
    )

    resp = await cluster.query.cluster_state(cluster_contract_address=cluster)
    print(resp)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_terra_ecosystem())
