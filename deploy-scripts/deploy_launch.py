import os

os.environ["USE_TEQUILA"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'

from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, store_contract
import asyncio
from base import deployer
from constants import graphql_mir_data, DEPLOY_ENVIRONMENT_STATUS_W_GOV


REQUIRE_GOV = True


async def deploy_new_incentives():

    print('Just initialize incentives')
    incentives_code_id = await store_contract("nebula_incentives")
    print('new code id', incentives_code_id)

    ecosystem = Ecosystem(require_gov=REQUIRE_GOV)

    ecosystem.terraswap_factory = Contract(
        "terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf"
    )

    for key in DEPLOY_ENVIRONMENT_STATUS_W_GOV:
        setattr(ecosystem, key, DEPLOY_ENVIRONMENT_STATUS_W_GOV[key])

    new_incentives = await Contract.create(
        incentives_code_id,
        owner=deployer.key.acc_address,
        factory=ecosystem.factory,
        terraswap_factory=ecosystem.terraswap_factory,
        nebula_token=ecosystem.neb_token,
        custody=ecosystem.incentives_custody,
        base_denom="uusd",
    )

    print('New incentives contract', new_incentives)
    # stupid name mangling
    await ecosystem.incentives_custody.__getattr__("__reset_owner")(
        owner=new_incentives
    )

    print(ecosystem.__dict__)

ADDR1 = "terra149xt9vmvmk9xag5f9zlnhqdw8yr8xu5kqmtyyk"
ADDR2 = "terra1hpwskqv92r6apn90kx3k9zk756g9j6m6zh4hmj"
ADDR3 = deployer.key.acc_address

async def deploy_mir_token_contracts():

    print('Deploying token contracts now')
    symbols_to_mir_contract = {}
    
    # ANC first
    # anc_contract = await Contract.create(
    #     DEPLOY_ENVIRONMENT_STATUS_W_GOV['code_ids']['terraswap_token'],
    #     name='Anchor',
    #     symbol='ANC',
    #     decimals=6,
    #     initial_balances=[
    #         {
    #             "address": ADDR1,
    #             "amount": "1" + "0" * 15,
    #         },
    #         {
    #             "address": ADDR2,
    #             "amount": "1" + "0" * 15,
    #         },
    #         {
    #             "address": ADDR3,
    #             "amount": "1" + "0" * 15,
    #         },
    #     ],
    #     mint=None,
    # )
    # symbols_to_mir_contract['ANC'] = anc_contract

    import pdb; pdb.set_trace()


    for asset in graphql_mir_data['data']['assets']:
        symbol, name, token = asset['symbol'], asset['name'], asset['token']

        mirror_contract = await Contract.create(
            DEPLOY_ENVIRONMENT_STATUS_W_GOV['code_ids']['terraswap_token'],
            name=name,
            symbol=symbol,
            decimals=6,
            initial_balances=[
                {
                    "address": ADDR1,
                    "amount": "1" + "0" * 15,
                },
                {
                    "address": ADDR2,
                    "amount": "1" + "0" * 15,
                },
                {
                    "address": ADDR3,
                    "amount": "1" + "0" * 15,
                },
            ],
            mint=None,
        )
        symbols_to_mir_contract[symbol] = mirror_contract

    print(symbols_to_mir_contract)


async def deploy_token_contracts():

    print('Deploying ERC20 contracts now')
    symbols_to_contracts = {}
    contracts_to_symbols = {}

    tokens = ["AAVE", "COMP", "MKR", "CREAM", "ANC", "DOGE", "ERCTWENTY", "CUMMIES", "MEME"]
    import pdb; pdb.set_trace()


    for t in tokens:
        symbol, name = t, t

        contract = await Contract.create(
            DEPLOY_ENVIRONMENT_STATUS_W_GOV['code_ids']['terraswap_token'],
            name=name,
            symbol=symbol,
            decimals=6,
            initial_balances=[
                {
                    "address": ADDR1,
                    "amount": "1" + "0" * 15,
                },
                {
                    "address": ADDR2,
                    "amount": "1" + "0" * 15,
                },
                {
                    "address": ADDR3,
                    "amount": "1" + "0" * 15,
                },
            ],
            mint=None,
        )
        symbols_to_contracts[symbol] = contract.address
        contracts_to_symbols[symbol] = contract.address
        

    print(symbols_to_contracts)
    print(contracts_to_symbols)

async def deploy_contracts():
    # await deploy_new_incentives()
    await deploy_token_contracts()

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_contracts())
