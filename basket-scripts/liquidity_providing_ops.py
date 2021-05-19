import time
from terra_sdk.client.lcd import LCDClient
from terra_sdk.client.localterra import LocalTerra
from terra_sdk.core.auth import StdFee
from terra_sdk.core.wasm import (
    MsgStoreCode,
    MsgInstantiateContract,
    MsgExecuteContract,
)
from terra_sdk.util.contract import get_code_id, get_contract_address, read_file_as_b64

from basket import Oracle, Basket, CW20, Asset, Governance
import pprint

from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
terra = get_terra()
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"

def lp_staking_queries(lp_token, staking_contract, basket_token, factory_contract, nebula_token):
    # LP STAKING FOR NEB REWARDS
    lp_tokens = terra.wasm.contract_query(
        lp_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - LP token balance after adding liquidity {lp_tokens}")

    print(f"[deploy] - bond lp tokens to staking contract")
    execute_contract(
        deployer,
        lp_token,
        CW20.send(staking_contract, "100", {"bond": {"asset_token": basket_token}}),
        seq(),
    )

    print(f"[deploy] - asking factory contract to distribute rewards")
    resp = execute_contract(deployer, factory_contract, {"distribute": {}}, seq())

    print(f"[deploy] - withdraw reward from staking contract")

    execute_contract(
        deployer, staking_contract, {"withdraw": {"asset_token": basket_token}}, seq()
    )

    neb_balance = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(
        f"[deploy] - nebula token balance after withdrawing staking reward - {neb_balance}"
    )