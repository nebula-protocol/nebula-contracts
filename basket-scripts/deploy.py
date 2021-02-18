"""Sample deploy script.

NOTE: Normally, we can use fee estimation in Tequila, as well as rely on Wallet to auto
fetch the sequence number from the blockchain. Here, we have manual options for sequence
number and fee.

Why manually incrementing sequence number: tequila endpoint is load-balanced so in successive
transactions, the nodes may not have had time to catch up to each other, which may result
in a signature (chain id, account, sequence) mismatch.

Why manually setting fee: tequila node allows simulating (auto-estimating) fee up to
3000000 gas. Some transactions such as code uploads and burning basket token (which
incurs multiple CW20 transfers to the user may require more gas than permitted by the
fee estimation feature).
"""

from terra_sdk.client.lcd import LCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
    dict_to_b64,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = False

lt = LocalTerra()

if USE_LOCALTERRA:
    terra = lt
    deployer = lt.wallets["test1"]
else:
    gas_prices = {
        "uluna": "0.15",
        "usdr": "0.1018",
        "uusd": "0.15",
        "ukrw": "178.05",
        "umnt": "431.6259",
        "ueur": "0.125",
        "ucny": "0.97",
        "ujpy": "16",
        "ugbp": "0.11",
        "uinr": "11",
        "ucad": "0.19",
        "uchf": "0.13",
        "uaud": "0.19",
        "usgd": "0.2",
    }

    terra = LCDClient(
        "https://tequila-fcd.terra.dev", "tequila-0004", gas_prices=gas_prices
    )
    deployer = terra.wallet(lt.wallets["test1"].key)

print(f"DEPLOYING WITH ACCCOUNT: {deployer.key.acc_address}")


def store_contract(contract_name, sequence):
    contract_bytes = read_file_as_b64(f"../artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)
    store_code_tx = deployer.create_and_sign_tx(
        msgs=[store_code], fee=StdFee(5000000, "2000000uluna"), sequence=sequence
    )
    result = terra.tx.broadcast(store_code_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_code_id(result)


def instantiate_contract(code_id, init_msg, sequence):
    instantiate = MsgInstantiateContract(deployer.key.acc_address, code_id, init_msg)
    instantiate_tx = deployer.create_and_sign_tx(
        msgs=[instantiate], sequence=sequence, denoms=["uluna"]
    )
    result = terra.tx.broadcast(instantiate_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_contract_address(result)


def execute_contract(wallet, contract_address, execute_msg, sequence, fee=None):
    execute = MsgExecuteContract(wallet.key.acc_address, contract_address, execute_msg)
    execute_tx = wallet.create_and_sign_tx(
        msgs=[execute], sequence=sequence, denoms=["uluna"], fee=fee
    )
    result = terra.tx.broadcast(execute_tx)
    if result.is_tx_error():
        print(result.raw_log)
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


sequence = deployer.sequence()


def seq():
    """Increments global sequence."""
    global sequence
    sequence += 1
    return sequence - 1


print(f"[main] - store terraswap_token")
token_code_id = store_contract("terraswap_token", seq())

print(f"[main] - store basket_dummy_oracle")
oracle_code_id = store_contract("basket_dummy_oracle", seq())

print(f"[main] - store basket_contract")
basket_code_id = store_contract("basket_contract", seq())

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
    seq(),
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
    seq(),
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
    seq(),
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
    seq(),
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
    seq(),
)

# instantiate oracle
print(f"[main] - instantiate oracle")
oracle = instantiate_contract(oracle_code_id, {}, seq())

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
    seq(),
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
    seq(),
)

# set basket token
print(f"[main] - set basket token")
execute_contract(deployer, basket, Basket.set_basket_token(basket_token), seq())

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
    seq(),
)

total = 5000000
amount_wBTC = get_amount(total * 0.08, "30000.0")
amount_wETH = get_amount(total * 0.18, "1500.0")
amount_wXRP = get_amount(total * 0.2, "0.45")
amount_wLUNA = get_amount(total * 0.2, "2.1")
amount_MIR = get_amount(total * 0.2, "5.06")
print(
    f"[main] - give initial balances wBTC {amount_wBTC} wETH {amount_wETH} wXRP {amount_wXRP} wLUNA {amount_wLUNA} MIR {amount_MIR}"
)
initial_balances_tx = deployer.create_and_sign_tx(
    msgs=[
        MsgExecuteContract(
            deployer.key.acc_address, wBTC, CW20.transfer(basket, amount_wBTC)
        ),
        MsgExecuteContract(
            deployer.key.acc_address, wETH, CW20.transfer(basket, amount_wETH)
        ),
        MsgExecuteContract(
            deployer.key.acc_address, wXRP, CW20.transfer(basket, amount_wXRP)
        ),
        MsgExecuteContract(
            deployer.key.acc_address, wLUNA, CW20.transfer(basket, amount_wLUNA)
        ),
        MsgExecuteContract(
            deployer.key.acc_address, MIR, CW20.transfer(basket, amount_MIR)
        ),
    ],
    sequence=seq(),
    fee=StdFee(4000000, "2000000uluna"),
)

result = terra.tx.broadcast(initial_balances_tx)

### EXAMPLE: how to stage and mint
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
    ],
    sequence=seq(),
    fee=StdFee(4000000, "2000000uluna"),
)

result = terra.tx.broadcast(stage_and_mint_tx)

### EXAMPLE: how to query
print(
    terra.wasm.contract_query(
        basket_token, {"balance": {"address": deployer.key.acc_address}}
    )
)

### EXAMPLE: how to burn
print("[main] - basket:burn")
burn = execute_contract(
    deployer,
    basket_token,
    CW20.send(basket, "1000000", Basket.burn()),
    seq(),
    fee=StdFee(4000000, "20000000uluna"),  # burning probably requires gas > 3000000
)

print(burn.txhash)

print(
    terra.wasm.contract_query(
        basket_token, {"balance": {"address": deployer.key.acc_address}}
    )
)

### EXAMPLE: how to query basket state
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