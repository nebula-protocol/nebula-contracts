import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio
from base import deployer
from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV, CONTRACT_TOKEN_TO_SYM_TEQ
import json
import requests

REQUIRE_GOV = True


async def deploy_momentum():

    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)

    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    for key in DEPLOY_ENVIRONMENT_STATUS_W_GOV:
        setattr(ecosystem, key, DEPLOY_ENVIRONMENT_STATUS_W_GOV[key])

    # Just include MIR for now, and rely on recomp to do things
    asset_tokens = [
        Contract("terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd"),  # MIR
    ]

    code_ids = ecosystem.code_ids
    oracle = await Contract.create(
        code_ids["nebula_dummy_oracle"],
        terraswap_factory=ecosystem.terraswap_factory,
        base_denom="uusd",
    )
    print('dummy pricing oracle', oracle)
    
    penalty_params = {
        "penalty_amt_lo": "0.1",
        "penalty_cutoff_lo": "0.01",
        "penalty_amt_hi": "0.5",
        "penalty_cutoff_hi": "0.1",
        "reward_amt": "0.05",
        "reward_cutoff": "0.02",
    }

    # Weight equally at first then can wait for rebalance to simplify things
    target_weights = [1]

    penalty_contract = await Contract.create(
        code_ids["nebula_penalty"],
        penalty_params=penalty_params,
        owner=ecosystem.factory,
    )

    create_cluster = ecosystem.factory.create_cluster(
        params={
            "name": "Top 5 30-Day Momentum",
            "symbol": "MOMENTUM",
            "penalty": penalty_contract,
            "target": target_weights,
            "assets": [Asset.cw20_asset_info(i.address) for i in asset_tokens],
            "pricing_oracle": oracle,
            "target_oracle": 'terra14ew659y4fn4dytu832k9f6l2u94668uclrywfg',
        },
    )

    if REQUIRE_GOV:
        # only need to send once
        gov_config = await ecosystem.gov.query.config()
        staker_info = await ecosystem.gov.query.staker(address=deployer.key.acc_address)
        print(staker_info)

        if float(staker_info['share']) == 0.0:
            await ecosystem.neb_token.send(
                contract=ecosystem.gov,
                amount="600000000000",
                msg=ecosystem.gov.stake_voting_tokens(lock_for_weeks = 104),
            )
            staker_info = await ecosystem.gov.query.staker(address=deployer.key.acc_address)
            print('post send', staker_info)

        resp = await ecosystem.create_and_execute_poll(
            {"contract": ecosystem.factory, "msg": create_cluster}, sleep_time=30
        )

    else:
        resp = await create_cluster

    logs = resp.logs[0].events_by_type

    instantiation_logs = logs["instantiate_contract"]
    addresses = instantiation_logs["contract_address"]

    cluster_token = Contract(addresses[2])
    cluster_pair = Contract(addresses[1])
    lp_token = Contract(addresses[0])

    # Use this because cw20
    cluster = ClusterContract(
        addresses[3],
        cluster_token,
        asset_tokens,
    )

    print("cluster", cluster)


    # Set prices so we can see if cluster deploys properly
    addresses = list(CONTRACT_TOKEN_TO_SYM_TEQ.keys())
    prices = [
        (a, "1") for a in addresses if a is not None
    ]

    # Will use pricing bot to set prices later
    await oracle.set_prices(prices=prices)

    print('setting prices')

    # await cluster.mint(asset_amounts=["1000"], min_tokens="100")

    # print('mint complete')

    resp = await cluster.query.cluster_state(cluster_contract_address=cluster)
    print(resp)

    print(logs)

    print("account", deployer.key.acc_address)
    
    print("assets", asset_tokens)
    print("ecosystem", ecosystem.__dict__)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_momentum())
