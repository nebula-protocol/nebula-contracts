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

from basket import Oracle, Basket, CW20, Asset, Governance, Community, Airdrop
import pprint

from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
terra = get_terra()
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"

def airdrop_operation(nebula_token, airdrop_contract):
    # give airdrop contract funds - should be covered by genesis
    print(f"[deploy] - initial funds to airdrop contract")
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, nebula_token, CW20.transfer(airdrop_contract, "10000000")
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)
    print(result.logs[0].events_by_type)

    stage = 1
    claim_amount = "1000000"
    proof = [
        "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42",
        "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f",
        "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3",
        "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2"
    ]

    print(f"[deploy] - register merkle root")
    resp = execute_contract(
        deployer,
        airdrop_contract,
        Airdrop.register_merkle_root("3b5c044802c4b768492f98fbd5a9253ed3dd97f5ff129de79a179249e2021766"),
        seq(),
    )
    print(resp.logs[0].events_by_type)

    print(f"[deploy] - claim airdrop")
    resp = execute_contract(
        deployer,
        airdrop_contract,
        Airdrop.claim(stage, claim_amount, proof),
        seq(),
    )
    print(resp.logs[0].events_by_type)