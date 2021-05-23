from basket import Oracle, Basket, CW20, Asset, Governance
from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids
from terra_sdk.core.auth import StdFee
import time

DEFAULT_PROPOSAL_DEPOSIT = "10000000000"
STAKE_AMOUNT = "500000000000"

poll_id = 1

deployer = get_deployer()
terra = get_terra()

def create_and_vote_poll(deployer, nebula_token, msg, gov_contract, collector_distribution=False, collector_contract=None):
    global poll_id
    print(f"[deploy] - current poll id {poll_id}")

    poll = Governance.create_poll(
        "Test",
        "Test",
        "TestLink1234",
        msg,
    )
    

    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(gov_contract, DEFAULT_PROPOSAL_DEPOSIT, poll),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)

    print(f"[deploy] - cast vote for YES")

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.cast_vote(poll_id, "yes", STAKE_AMOUNT),
        seq(),
        fee=StdFee(4000000, "200000uluna"),
    )

    print(result.logs[0].events_by_type)

    if collector_distribution:
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

    # ensure poll results are reflected on chain
    time.sleep(5)

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.end_poll(poll_id),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.execute_poll(poll_id),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)

    poll_id += 1
    
    return result