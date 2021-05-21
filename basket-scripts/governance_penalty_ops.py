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
from governance_helpers import create_and_vote_poll
import pprint

from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
terra = get_terra()
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"


def create_new_penalty_with_gov(nebula_token, gov_contract, penalty_code_id, collector_contract, basket):
    # GOVERNANCE VOTING FOR NEB REWARDS

    print(f"[deploy] - create new penalty function via governance")

    new_penalty_contract = instantiate_contract(
        penalty_code_id,
        {
            "penalty_params": {
                "penalty_amt_lo": "0.2",
                "penalty_cutoff_lo": "0.01",
                "penalty_amt_hi": "0.5",
                "penalty_cutoff_hi": "0.1",
                "reward_amt": "0.05",
                "reward_cutoff": "0.02",
            },
            "owner": basket,
        },
        seq(),
    )

    poll_msg = Governance.create_execute_msg(
        basket, Basket.reset_penalty(new_penalty_contract)
    )

    result = create_and_vote_poll(deployer, nebula_token, poll_msg, gov_contract, collector_distribution=True, collector_contract=collector_contract)

    #query relevant balances post penalty change through governance

    nebula_tokens = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - neb balance before withdrawing voting rewards {nebula_tokens}")

    execute_contract(deployer, gov_contract, {"withdraw_voting_rewards": {}}, seq())

    nebula_tokens = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": deployer.key.acc_address}}
    )

    print(f"[deploy] - neb balance after withdrawing voting rewards {nebula_tokens}")