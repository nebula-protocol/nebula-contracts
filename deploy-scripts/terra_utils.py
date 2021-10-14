import asyncio
import os
from terra_sdk.key.mnemonic import MnemonicKey
from terra_sdk.client.lcd import AsyncLCDClient
from terra_sdk.client.localterra import AsyncLocalTerra
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.core.bank import MsgSend
from terra_sdk.util.contract import get_code_id, read_file_as_b64
import os


CONTRACT_DIR = os.path.join(
    os.path.dirname(os.path.dirname(__file__)),
    "contracts",
    "artifacts",
)


def custom_objs_to_json(obj):
    if type(obj) == dict:
        return {k: custom_objs_to_json(v) for k, v in obj.items()}
    if type(obj) in {list, tuple}:
        return [custom_objs_to_json(i) for i in obj]
    # contract objects
    if hasattr(obj, "address"):
        return obj.address
    # executemessage objects
    if hasattr(obj, "json"):
        return obj.json
    return obj


class Account:
    def __init__(self, tequila=False, key=None):

        lt = AsyncLocalTerra(gas_prices={"uusd": "0.15"})
        if tequila:

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

            self.terra = AsyncLCDClient(
                "https://tequila-fcd.terra.dev", "tequila-0004", gas_prices=gas_prices
            )
            if key is None:
                raise Exception("Please enter a key")

            self.deployer = self.terra.wallet(key)
        else:
            if key is None:
                key = "test1"
            self.terra = lt
            self.deployer = lt.wallets[key]

        self.key = self.deployer.key
        self.acc_address = self.key.acc_address
        self.sequence = None

        outer_obj = self

        class Message:
            def __init__(self):
                self.msg = None

            def __await__(self):
                return outer_obj.sign_and_broadcast(self.msg).__await__()

        class ExecuteMessage(Message):
            def __init__(self, contract, json, send=None):
                super(ExecuteMessage, self).__init__()
                self.contract = contract
                self.json = custom_objs_to_json(json)
                self.msg = MsgExecuteContract(
                    outer_obj.acc_address, self.contract.address, self.json, send
                )

        class InstantiateMessage(Message):
            def __init__(self, code_id, json, init_coins=None):
                super(InstantiateMessage, self).__init__()
                self.json = custom_objs_to_json(json)
                self.msg = MsgInstantiateContract(
                    outer_obj.acc_address,
                    outer_obj.acc_address,
                    code_id,
                    self.json,
                    init_coins=init_coins,
                )

        class SendMsg(Message):
            def __init__(self, recipient, amount):
                super(SendMsg, self).__init__()
                self.msg = MsgSend(
                    amount=amount,
                    to_address=recipient,
                    from_address=outer_obj.acc_address,
                )

        class Contract:
            def __init__(self, address):
                self.address = address

            @classmethod
            async def create(cls, code_id, init_coins=None, **kwargs):
                msg = InstantiateMessage(code_id, kwargs, init_coins=init_coins)
                result = await msg
                if result.logs:
                    contract_address = result.logs[0].events_by_type[
                        "instantiate_contract"
                    ]["contract_address"][-1]
                    return cls(contract_address)
                else:
                    raise ValueError("could not parse code id -- tx logs are empty.")

            @property
            def query(self):
                contract_addr = self.address

                class ContractQuerier:
                    def __getattr__(self, item):
                        async def result_fxn(**kwargs):
                            kwargs = custom_objs_to_json(kwargs)
                            return await outer_obj.terra.wasm.contract_query(
                                contract_addr, custom_objs_to_json({item: kwargs})
                            )

                        return result_fxn

                return ContractQuerier()

            def __getattr__(self, item):
                def result_fxn(_send=None, **kwargs):
                    return ExecuteMessage(
                        contract=self, json={item: kwargs}, send=_send
                    )

                return result_fxn

        self.send = SendMsg
        self.execute = ExecuteMessage
        self.contract = Contract

    async def store_contracts(self):

        contract_names = [
            i[:-5] for i in os.listdir(CONTRACT_DIR) if i.endswith(".wasm")
        ]
        return {
            contract_name: await self.store_contract(contract_name)
            for contract_name in contract_names
        }

    async def store_contract(self, contract_name):

        contract_bytes = read_file_as_b64(f"{CONTRACT_DIR}/{contract_name}.wasm")
        store_code = MsgStoreCode(self.acc_address, contract_bytes)

        result = await self.sign_and_broadcast(store_code)
        code_id = get_code_id(result)
        print(f"Code id for {contract_name} is {code_id}")
        return code_id

    async def chain(self, *messages):
        return await self.sign_and_broadcast(*[i.msg for i in messages])

    async def sign_and_broadcast(self, *msgs):
        if self.sequence is None:
            self.sequence = await self.deployer.sequence()

        try:
            tx = await self.deployer.create_and_sign_tx(
                msgs=msgs,
                gas_prices={"uusd": "0.15"},
                gas_adjustment=1.5,
                sequence=self.sequence,
            )
            result = await self.terra.tx.broadcast(tx)
            self.sequence += 1
            if result.is_tx_error():
                raise Exception(result.raw_log)
            return result
        except:
            self.sequence = await self.deployer.sequence()
            raise

    async def __aenter__(self):
        await self.terra.__aenter__()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.terra.__aexit__(exc_type, exc_val, exc_tb)


# class ClusterContract(Contract):
#     def __init__(self, address, cluster_token, asset_tokens):
#         super().__init__(address)
#         self.cluster_token = cluster_token
#         self.asset_tokens = asset_tokens

#     def __repr__(self):
#         return f'ClusterContract("{self.address}", {self.cluster_token}, {self.asset_tokens})'

#     async def mint(self, asset_amounts, min_tokens=None):
#         msgs = []
#         mint_assets = []
#         for asset, amt in zip(self.asset_tokens, asset_amounts):
#             msgs.append(asset.increase_allowance(spender=self, amount=amt))
#             mint_assets.append(Asset.asset(asset, amt))

#         msgs.append(
#             self.__getattr__("mint")(asset_amounts=mint_assets, min_tokens=min_tokens)
#         )
#         return await chain(*msgs)

#     async def redeem(self, max_tokens, asset_amounts=None):
#         msgs = [self.cluster_token.increase_allowance(spender=self, amount=max_tokens)]

#         if asset_amounts is not None:
#             asset_amounts = [
#                 Asset.asset(i, amt) for i, amt in zip(self.asset_tokens, asset_amounts)
#             ]

#         msgs.append(
#             self.burn(
#                 max_tokens=max_tokens,
#                 asset_amounts=asset_amounts,
#             )
#         )
#         return await chain(*msgs)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(test())
