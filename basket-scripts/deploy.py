from terra_sdk.client.lcd import LCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
    dict_to_b64,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

lt = LocalTerra()

if USE_LOCALTERRA:
    terra = lt
    deployer = lt.wallets["test1"]
else:
    terra = LCDClient("https://tequila-fcd.terra.dev", "tequila-0004")
    deployer = terra.wallet(lt.wallets["test1"].key)


def store_contract(contract_name):
    contract_bytes = read_file_as_b64(f"../artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)
    store_code_tx = deployer.create_and_sign_tx(msgs=[store_code])
    result = terra.tx.broadcast(store_code_tx)
    return get_code_id(result)


def instantiate_contract(code_id, init_msg):
    instantiate = MsgInstantiateContract(deployer.key.acc_address, code_id, init_msg)
    instantiate_tx = deployer.create_and_sign_tx(msgs=[instantiate])
    result = terra.tx.broadcast(instantiate_tx)
    return get_contract_address(result)


def execute_contract(wallet, contract_address, execute_msg):
    execute = MsgExecuteContract(wallet.key.acc_address, contract_address, execute_msg)

    execute_tx = wallet.create_and_sign_tx(msgs=[execute])

    result = terra.tx.broadcast(execute_tx)
    return result


def get_amount(value, price):
    """Gets Uint128 amount of coin in order to get total value, assuming price."""
    return str(int(value / float(price) * 1000000))


class Oracle:
    @staticmethod
    def set_prices(prices):
        return {"set_prices": {"prices": prices}}


class CW20:
    @staticmethod
    def transfer(recipient, amount):
        return {"transfer": {"recipient": recipient, "amount": amount}}

    @staticmethod
    def send(contract, amount, msg_data=None):
        msg = None
        if msg_data is not None:
            msg = dict_to_b64(msg_data)
        return {"send": {"contract": contract, "amount": amount, "msg": msg}}


class Basket:
    @staticmethod
    def set_basket_token(basket_token):
        return {"__set_basket_token": {"basket_token": basket_token}}

    @staticmethod
    def mint(asset_amounts):
        return {"mint": {"asset_amounts": asset_amounts}}

    @staticmethod
    def stage_asset():
        return {"stage_asset": {}}

    @staticmethod
    def burn(asset_weights=None):
        return {"burn": {"asset_weights": asset_weights}}


token_code_id = store_contract("terraswap_token")
oracle_code_id = store_contract("basket_dummy_oracle")
basket_code_id = store_contract("basket_contract")

# wrapped bitcoin
print(f"[main] - instantiate wBTC")
wBTC = instantiate_contract(
    token_code_id,
    {
        "name": "Wrapped Bitcoin",
        "symbol": "wBTC",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "400000000"}
        ],
        "mint": None,
    },
)

# wrapped ether
print(f"[main] - instantiate wETH")
wETH = instantiate_contract(
    token_code_id,
    {
        "name": "Wrapped Ethereum",
        "symbol": "wETH",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "20000000000"}
        ],
        "mint": None,
    },
)

# wrapped ripple
print(f"[main] - instantiate wXRP")
wXRP = instantiate_contract(
    token_code_id,
    {
        "name": "Wrapped Ripple",
        "symbol": "wXRP",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "5000000000000"}
        ],
        "mint": None,
    },
)

# wrapped luna
print(f"[main] - instantiate wLUNA")
wLUNA = instantiate_contract(
    token_code_id,
    {
        "name": "Wrapped Luna",
        "symbol": "wLUNA",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "1000000000000"}
        ],
        "mint": None,
    },
)

# mirror token
print(f"[main] - instantiate MIR")
MIR = instantiate_contract(
    token_code_id,
    {
        "name": "Mirror Token",
        "symbol": "MIR",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "1000000000000"}
        ],
        "mint": None,
    },
)

# instantiate oracle
print(f"[main] - instantiate oracle")
oracle = instantiate_contract(oracle_code_id, {})

# instantiate basket
print(f"[main] - instantiate basket")
basket = instantiate_contract(
    basket_code_id,
    {
        "name": "Basket",
        "owner": deployer.key.acc_address,
        "assets": [wBTC, wETH, wXRP, wLUNA, MIR],
        "oracle": oracle,
        "penalty_params": {
            "a_pos": "1",
            "s_pos": "1",
            "a_neg": "0.005",
            "s_neg": "0.5",
        },
        "target": [10, 20, 15, 30, 25],
    },
)

# instantiate basket token
print(f"[main] - instantiate basket token")
basket_token = instantiate_contract(
    token_code_id,
    {
        "name": "Basket Token",
        "symbol": "BASKET",
        "decimals": 6,
        "initial_balances": [
            {"address": deployer.key.acc_address, "amount": "1000000000000"}
        ],
        "mint": {"minter": basket, "cap": None},
    },
)

# set basket token
print(f"[main] - set basket token")
execute_contract(deployer, basket, Basket.set_basket_token(basket_token))

# set oracle prices
print(f"[main] - set oracle prices")
execute_contract(
    deployer,
    oracle,
    Oracle.set_prices(
        [
            [wBTC, "30000.0"],
            [wETH, "1500.0"],
            [wXRP, "0.45"],
            [wLUNA, "2.1"],
            [MIR, "5.06"],
        ]
    ),
)

total = 5000000
amount_wBTC = get_amount(total * 0.08, "30000.0")
print(f"[main] - give initial balance of wBTC {amount_wBTC}")
execute_contract(deployer, wBTC, CW20.transfer(basket, amount_wBTC))

amount_wETH = get_amount(total * 0.18, "1500.0")
print(f"[main] - give initial balance of wETH {amount_wETH}")
execute_contract(deployer, wETH, CW20.transfer(basket, amount_wETH))

amount_wXRP = get_amount(total * 0.2, "0.45")
print(f"[main] - give initial balance of wXRP {amount_wXRP}")
execute_contract(deployer, wXRP, CW20.transfer(basket, amount_wXRP))

amount_wLUNA = get_amount(total * 0.2, "2.1")
print(f"[main] - give initial balance of wLUNA {amount_wLUNA}")
execute_contract(deployer, wXRP, CW20.transfer(basket, amount_wLUNA))

amount_MIR = get_amount(total * 0.2, "5.06")
print(f"[main] - give initial balance of MIR {amount_MIR}")
execute_contract(deployer, wXRP, CW20.transfer(basket, amount_MIR))


print("[main] - basket:burn")
execute_contract(deployer, MIR, CW20.transfer(basket, get_amount(total * 0.22, "5.06")))

print("[main] - basket:stage_asset + basket:mint")
stage_and_mint_tx = deployer.create_and_sign_tx(
    msgs=[
        MsgExecuteContract(
            deployer.key.acc_address,
            wBTC,
            CW20.send(basket, "1000000", Basket.stage_asset()),
        ),
        MsgExecuteContract(
            deployer.key.acc_address,
            wLUNA,
            CW20.send(basket, "4000000000", Basket.stage_asset()),
        ),
        MsgExecuteContract(
            deployer.key.acc_address,
            basket,
            Basket.mint(["1000000", "0", "0", "4000000000", "0"]),
        ),
    ]
)

result = terra.tx.broadcast(stage_and_mint_tx)

print("[main] - basket:burn")
burn = execute_contract(
    deployer, basket_token, CW20.send(basket, "10000000", Basket.burn([1, 1, 1, 9, 2]))
)

print(
    terra.wasm.contract_query(
        basket, {"basket_state": {"basket_contract_address": basket}}
    )
)

print(
    {
        "wBTC": wBTC,
        "wETH": wETH,
        "wXRP": wXRP,
        "wLUNA": wLUNA,
        "MIR": MIR,
        "oracle": oracle,
        "basket": basket,
    }
)