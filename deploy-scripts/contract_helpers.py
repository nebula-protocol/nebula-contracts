from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.json import dict_to_data
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64
from base import (
    deployer,
    sign_and_broadcast,
    OVERWRITE_CACHE_ALLOWED,
    CACHE_INITIALIZATION,
    terra,
    USE_BOMBAY,
)
from api import Asset
import shelve
import os
import base64
import json

shelf = shelve.open(f"{os.path.dirname(__file__)}/cache.dat")


def async_cache_on_disk(fxn):
    async def _ret(*args, **kwargs):
        key = repr(args) + "|" + repr(kwargs)
        key = str(USE_BOMBAY) + "|" + fxn.__name__ + "|" + str(key)
        if key not in shelf or fxn.__name__ in OVERWRITE_CACHE_ALLOWED:
            shelf[key] = await fxn(*args, **kwargs)
            shelf.sync()
        return shelf[key]

    return _ret if CACHE_INITIALIZATION else fxn


@async_cache_on_disk
async def store_contract(contract_name):

    parent_dir = os.path.dirname(os.path.dirname(__file__))
    contract_bytes = read_file_as_b64(f"{parent_dir}/artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)

    result = await sign_and_broadcast(store_code)
    code_id = get_code_id(result)
    print(f"Code id for {contract_name} is {code_id}")
    return code_id


async def store_contracts():

    parent_dir = os.path.dirname(os.path.dirname(__file__))
    contract_names = [
        i[:-5] for i in os.listdir(f"{parent_dir}/artifacts") if i.endswith(".wasm")
    ]
    return {
        contract_name: await store_contract(contract_name)
        for contract_name in contract_names
    }


async def instantiate_token_contract(token_code_id, name, symbol, initial_amount):
    return await Contract.create(
        token_code_id,
        name=name,
        symbol=symbol,
        decimals=6,
        initial_balances=[
            {"address": deployer.key.acc_address, "amount": initial_amount}
        ],
        mint=None,
    )


async def chain(*messages):
    return await sign_and_broadcast(*[i.msg for i in messages])


class ExecuteMessage:
    def __init__(self, contract, json, send=None):
        self.contract = contract
        self.json = custom_objs_to_json(json)
        self.msg = MsgExecuteContract(
            deployer.key.acc_address, self.contract.address, self.json, send
        )

    def __await__(self):
        return sign_and_broadcast(self.msg).__await__()


class ContractQuerier:
    def __init__(self, address):
        self.address = address

    def __getattr__(self, item):
        async def result_fxn(**kwargs):
            kwargs = custom_objs_to_json(kwargs)
            return await terra.wasm.contract_query(self.address, {item: kwargs})

        return result_fxn


class Contract:
    @staticmethod
    async def create(code_id, **kwargs):
        kwargs = custom_objs_to_json(kwargs)
        instantiate = MsgInstantiateContract(deployer.key.acc_address, deployer.key.acc_address, code_id, kwargs)
        result = await sign_and_broadcast(instantiate)
        return Contract(get_contract_address(result))

    def __init__(self, address):
        self.address = address

    def __repr__(self):
        return f'Contract("{self.address}")'

    def __getattr__(self, item):
        def result_fxn(_send=None, **kwargs):
            return ExecuteMessage(contract=self, json={item: kwargs}, send=_send)
        return result_fxn

    @property
    def query(self):
        return ContractQuerier(self.address)


class ClusterContract(Contract):
    def __init__(self, address, cluster_token, asset_tokens):
        super().__init__(address)
        self.cluster_token = cluster_token
        self.asset_tokens = asset_tokens

    def __repr__(self):
        return f'ClusterContract("{self.address}", {self.cluster_token}, {self.asset_tokens})'

    async def mint(self, asset_amounts, min_tokens=None):
        msgs = []
        mint_assets = []
        for asset, amt in zip(self.asset_tokens, asset_amounts):
            msgs.append(asset.increase_allowance(spender=self, amount=amt))
            mint_assets.append(Asset.asset(asset, amt))

        await chain(*msgs)
        msgs = []

        msgs.append(
            self.__getattr__("rebalance_create")(asset_amounts=mint_assets, min_tokens=min_tokens)
        )
        return await chain(*msgs)

    async def redeem(self, max_tokens, asset_amounts=None):
        msgs = [self.cluster_token.increase_allowance(spender=self, amount=max_tokens)]

        if asset_amounts is not None:
            asset_amounts = [
                Asset.asset(i, amt) for i, amt in zip(self.asset_tokens, asset_amounts)
            ]

        msgs.append(
            self.rebalance_redeem(
                max_tokens=max_tokens,
                asset_amounts=asset_amounts,
            )
        )
        return await chain(*msgs)


def custom_objs_to_json(obj):
    if type(obj) == dict:
        return {k: custom_objs_to_json(v) for k, v in obj.items()}
    if type(obj) in {list, tuple}:
        return [custom_objs_to_json(i) for i in obj]
    if issubclass(type(obj), Contract):
        return obj.address
    if type(obj) == ExecuteMessage:
        return obj.json
        # return dict_to_data(obj.json)
    return obj

def dict_to_b64(data: dict) -> str:
    """Converts dict to ASCII-encoded base64 encoded string."""
    return base64.b64encode(bytes(json.dumps(data), "ascii")).decode()