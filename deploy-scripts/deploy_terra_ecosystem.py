import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, chain
import asyncio
from base import deployer
from constants import DEPLOY_ENVIRONMENT_STATUS_W_GOV, SYM_TO_CONTRACT_TOKEN_TEQ

REQUIRE_GOV = True


async def deploy_terra_ecosystem():

    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)

    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    for key in DEPLOY_ENVIRONMENT_STATUS_W_GOV:
        setattr(ecosystem, key, DEPLOY_ENVIRONMENT_STATUS_W_GOV[key])

    MIR_ADDR = SYM_TO_CONTRACT_TOKEN_TEQ['MIR'].address
    ANC_ADDR = SYM_TO_CONTRACT_TOKEN_TEQ['ANC'].address

    cw20_asset_tokens = [
        Contract(MIR_ADDR),  # MIR
        Contract(ANC_ADDR),  # ANC
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
    target_weights = [1, 1, 1]
    # target_weights = [1, 1]

    penalty_contract = await Contract.create(
        code_ids["nebula_penalty"],
        penalty_params=penalty_params,
        owner=ecosystem.factory,
    )

    # Asset tokens to include
    asset_tokens = [Asset.cw20_asset_info(i) for i in cw20_asset_tokens] + [Asset.native_asset_info('uluna')]

    create_cluster = ecosystem.factory.create_cluster(
        params={
            "name": "Terra Ecosystem",
            "symbol": "TER",
            "penalty": penalty_contract,
            "target": target_weights,
            "assets": asset_tokens,
            "pricing_oracle": oracle,
            "composition_oracle": 'terra1qyz9ps2dpv8ay4rg4hy65fvc3wjxu83s246tpy',
        },
    )

    if REQUIRE_GOV:
        # only need to send once
        gov_config = await ecosystem.gov.query.config()
        staker_info = await ecosystem.gov.query.staker(address=deployer.key.acc_address)
        print(staker_info)

        if float(staker_info['share']) < float(gov_config['quorum']):
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

    cluster_info = ClusterContract(
        addresses[3],
        cluster_token,
        asset_tokens,
    )

    print(cluster_info)

    cluster = Contract(
        addresses[3],
    )

    print("cluster", cluster)

    prices = [
        (MIR_ADDR, "1"),
        (ANC_ADDR, "1"),
        ("uluna", "1")
    ]

    await oracle.set_prices(prices=prices)

    print('setting prices')

    mint_cw20_assets = [
        Contract(MIR_ADDR),
        Contract(ANC_ADDR),
    ]

    # Do this separately because we have a native asset
    # msgs = []
    # mint_assets = []
    # for asset in mint_cw20_assets:
    #     msgs.append(asset.increase_allowance(spender=cluster, amount="1000"))
    #     mint_assets.append(Asset.asset(asset, "1000"))

    # mint_assets.append(Asset.asset("uluna", "1000", native=True))

    # msgs.append(
    #     cluster.mint(asset_amounts=mint_assets, min_tokens="100", _send={"uluna": "1000"})
    # )

    # await chain(*msgs)
    

    # print('mint complete')

    resp = await cluster.query.cluster_state(cluster_contract_address=cluster)
    print('cluster_state', resp)

    print(logs)

    print("account", deployer.key.acc_address)
    
    print("assets", asset_tokens)
    print("ecosystem", ecosystem.__dict__)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_terra_ecosystem())
