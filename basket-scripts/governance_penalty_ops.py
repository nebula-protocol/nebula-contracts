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


def create_poll_for_new_penalty_contract(penalty_code_id, nebula_token, gov_contract, basket):
    print(f"[deploy] - create new penalty contract")
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

    poll = Governance.create_poll(
        "Test",
        "Test",
        "TestLink1234",
        Governance.create_execute_msg(
            basket, Basket.reset_penalty(new_penalty_contract)
        ),
    )

    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(gov_contract, DEFAULT_PROPOSAL_DEPOSIT, poll),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)
    return result

def cast_vote_and_check_balances(gov_contract, collector_contract, nebula_token):
    print(f"[deploy] - cast vote for YES")
    stake_amount = "500000000000"
    result = execute_contract(
        deployer,
        gov_contract,
        Governance.cast_vote(2, "yes", stake_amount),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)


    print(f"sequence # is: {deployer.sequence()}")

    print(f"[deploy] - commissioner distributes neb to gov contract")
    execute_contract(deployer, collector_contract, {"distribute": {}}, seq())

    nebula_tokens = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": collector_contract}}
    )

    print(f"[deploy] - collector nebula balance {nebula_tokens}")
    nebula_tokens = terra.wasm.contract_query(
        nebula_token, {"balance": {"address": gov_contract}}
    )

    print(f"[deploy] - gov nebula balance {nebula_tokens}")
    time.sleep(2) #ensure that poll vote is reflected on chain


def end_poll_and_execute(gov_contract):
    print(f"[deploy] - execute vote")

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.end_poll(2),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )
    # execute poll
    result = execute_contract(
        deployer,
        gov_contract,
        Governance.execute_poll(2),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )


def create_new_penalty_with_gov(nebula_token, gov_contract, penalty_code_id, collector_contract, basket):
    # GOVERNANCE VOTING FOR NEB REWARDS

    print(f"[deploy] stake voting tokens")
    execute_contract(
        deployer,
        nebula_token,
        CW20.send(gov_contract, "100", {"stake_voting_tokens": {}}),
        seq(),
    )

    print(f"[deploy] - create poll")
    result = create_poll_for_new_penalty_contract(penalty_code_id, nebula_token, gov_contract, basket)


    # cast vote
    cast_vote_and_check_balances(gov_contract, collector_contract, nebula_token)

    #end poll and execute
    end_poll_and_execute(gov_contract)


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