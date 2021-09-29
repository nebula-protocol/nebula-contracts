import os

os.environ["USE_BOMBAY"] = "1"
os.environ["MNEMONIC"] = 'museum resist wealth require renew punch jeans smooth old color neutral cactus baby retreat guitar web average piano excess next strike drive game romance'
# os.environ["MNEMONIC"] = "canal tip borrow fly skirt auction volume scene great wrap wise album feature toast lawsuit ginger sweet cat reunion garlic early inspire napkin salt"
from api import Asset
from ecosystem import Ecosystem
from contract_helpers import Contract, ClusterContract, store_contract, chain
import asyncio
from base import deployer
from constants import graphql_mir_data, DEPLOY_ENVIRONMENT_STATUS_W_GOV, CONTRACT_TOKEN_TO_SYM_TEQ, CONTRACT_TOKEN_TO_SYM_BOMBAY_11


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

    print('Deploying mirror token contracts now')
    symbols_to_mir_contract = {}
    contract_to_symbol = {}
    for asset in graphql_mir_data['data']['assets']:
        print(asset)
        
        symbol, name, token = asset['symbol'], asset['name'], asset['token']
        try:
            mirror_contract = await Contract.create(
                # DEPLOY_ENVIRONMENT_STATUS_W_GOV['code_ids']['terraswap_token'],
                9462,
                name=name,
                symbol=symbol,
                decimals=6,
                initial_balances=[
                    {
                        "address": ADDR1,
                        "amount": "1" + "0" * 18,
                    },
                    {
                        "address": ADDR2,
                        "amount": "1" + "0" * 18,
                    },
                    {
                        "address": ADDR3,
                        "amount": "1" + "0" * 18,
                    },
                ],
                mint=None,
            )
            symbols_to_mir_contract[symbol] = mirror_contract.address
            contract_to_symbol[mirror_contract.address] = symbol
        except:
            print('broken', symbol)
            print(symbols_to_mir_contract)
            print(contract_to_symbol)

    print(symbols_to_mir_contract)
    print(contract_to_symbol)


async def deploy_token_contracts():

    print('Deploying ERC20 contracts now')
    symbols_to_contracts = {}
    contracts_to_symbols = {}

    tokens = ["AAVE", "COMP", "MKR", "CREAM", "ANC", "AXS", "SAND", "MANA", "ENJ", "AUDIO"]


    for t in tokens:
        symbol, name = t, t
        print(symbol)

        contract = await Contract.create(
            # DEPLOY_ENVIRONMENT_STATUS_W_GOV['code_ids']['terraswap_token'],
            9462,
            name=name,
            symbol=symbol,
            decimals=6,
            initial_balances=[
                {
                    "address": ADDR1,
                    "amount": "1" + "0" * 18,
                },
                {
                    "address": ADDR2,
                    "amount": "1" + "0" * 18,
                },
                {
                    "address": ADDR3,
                    "amount": "1" + "0" * 18,
                },
            ],
            mint=None,
        )
        symbols_to_contracts[symbol] = contract.address
        contracts_to_symbols[contract.address] = symbol
        

    print(symbols_to_contracts)
    print(contracts_to_symbols)


# SEND_TO = ['terra1hpwskqv92r6apn90kx3k9zk756g9j6m6zh4hmj','terra13dsvt7t99vv74nx45zegufm0yz8gu8n2wsx39l']

SEND_TO = ['terra12hnhh5vtyg5juqnzm43970nh4fw42pt27nw9g9', 'terra1jfa9uhs0z6rkpn2l5zy0jx3l8a8envht07gx3p']

async def quick_transfer():
    # SEND_TO = "terra149xt9vmvmk9xag5f9zlnhqdw8yr8xu5kqmtyyk"
    for s in SEND_TO:
        msgs = []
        for token, symbol in CONTRACT_TOKEN_TO_SYM_BOMBAY_11.items():
            print(symbol, token)
            contract = Contract(token)
            # transfer_out = str(10**15 - 10**6)
            transfer_out = str(10**12)
            msgs.append(contract.transfer(recipient=s, amount=transfer_out))
            
        await chain(*msgs)
        print("transferred out for", s)

async def provide_lp():
    terraswap_factory = Contract('terra18qpjm4zkvqnpjpw0zn0tdr8gdzvt8au35v45xf')

    neb_token = Contract('terra1ccthcaymaeatd0ty42mka3wxj36hgxm5r49446')

    asset_infos = [Asset.cw20_asset_info(neb_token.address), Asset.native_asset_info('uusd')] 
    pair_info = await terraswap_factory.query.pair(asset_infos=asset_infos)

    pair_contract = Contract(pair_info['contract_addr'])

    msgs = []

    provide_uusd = 500000000000 # Provide $500 liquidity

    cost_per_ct = 1
    provide_cluster_token = int(provide_uusd / cost_per_ct)
    print(provide_cluster_token)
    # Increase allowance
    msgs.append(neb_token.increase_allowance(spender=pair_contract, amount=str(provide_cluster_token)))

    # Provide liquidity
    assets = [Asset.asset(neb_token, str(provide_cluster_token)), Asset.asset('uusd', str(provide_uusd), native=True)]
    msgs.append(pair_contract.provide_liquidity(assets=assets, _send={"uusd": str(provide_uusd)}))
    await chain(*msgs)

async def deploy_contracts():
    # await deploy_new_incentives()
    await deploy_mir_token_contracts()
    await deploy_token_contracts()
    # await quick_transfer()
    # await provide_lp()

if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(deploy_contracts())
