import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio
from base import deployer
from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV, CONTRACT_TOKEN_TO_SYM_TEQ, CT_SYM_TO_NAME
import json
import requests

import sys

REQUIRE_GOV = True


async def deploy_cluster(ct_symbol, recomp_oracle):

    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)

    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    for key in DEPLOY_ENVIRONMENT_STATUS_W_GOV:
        setattr(ecosystem, key, DEPLOY_ENVIRONMENT_STATUS_W_GOV[key])

    code_ids = ecosystem.code_ids
    
    penalty_params = {
        "penalty_amt_lo": "0.1",
        "penalty_cutoff_lo": "0.01",
        "penalty_amt_hi": "1",
        "penalty_cutoff_hi": "0.1",
        "reward_amt": "0.05",
        "reward_cutoff": "0.02",
    }

    # Weight equally at first then can wait for rebalance to simplify things
    assets = [
        Asset.asset("terra1gkjll5uwqlwa8mrmtvzv435732tffpjql494fd", "1") # MIR
    ]

    penalty_contract = await Contract.create(
        code_ids["nebula_penalty"],
        penalty_params=penalty_params,
        owner=ecosystem.factory,
    )

    print("Trying to deploy", CT_SYM_TO_NAME[ct_symbol], ct_symbol)

    oracle = ecosystem.dummy_oracle

    create_cluster = ecosystem.factory.create_cluster(
        params={
            "name": CT_SYM_TO_NAME[ct_symbol],
            "description": f"Testing {ct_symbol} cluster",
            "symbol": ct_symbol,
            "penalty": penalty_contract,
            "target": assets,
            "pricing_oracle": oracle, # Generic pricing oracle
            "composition_oracle": recomp_oracle,
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
            {"contract": ecosystem.factory, "msg": create_cluster}, sleep_time=45
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
        None,
    )

    print("cluster", cluster)


    # Set prices so we can see if cluster deploys properly
    price_addresses = list(CONTRACT_TOKEN_TO_SYM_TEQ.keys())
    prices = [
        (a, "1") for a in price_addresses if a is not None
    ] + [("uusd", "1"), ("uluna", "1")]

    # Will use pricing bot to set prices later
    await oracle.set_prices(prices=prices)

    print('setting prices')

    # await cluster.mint(asset_amounts=["1000"], min_tokens="100")

    # print('mint complete')

    resp = await cluster.query.cluster_state(cluster_contract_address=cluster)
    print(resp)

    print(logs)

    print("deployer account", deployer.key.acc_address)
    
    print("assets", assets)
    # print("ecosystem", ecosystem.__dict__)

    cluster_list = await ecosystem.factory.query.cluster_list()
    print('cluster_list', cluster_list)

if __name__ == "__main__":
    ct_symbol = sys.argv[1]
    recomp_oracle = sys.argv[2]
    asyncio.get_event_loop().run_until_complete(deploy_cluster(ct_symbol, recomp_oracle))
