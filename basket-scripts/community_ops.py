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

from basket import Oracle, Basket, CW20, Asset, Governance, Community
import pprint

from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids

deployer = get_deployer()
terra = get_terra()
DEFAULT_PROPOSAL_DEPOSIT = "10000000000"

def community_operations(nebula_token, community_contract, gov_contract):
    # transfer some to community pool
    initial_community_neb_amt = "100000000"
    print(
        f"[deploy] - give community initial balance of nebula {initial_community_neb_amt}"
    )
    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, nebula_token, CW20.transfer(community_contract, initial_community_neb_amt)
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)

    # Create poll
    print(
        f"[deploy] - create poll to spend"
    )

    spend_amt = "10000"
    poll = Governance.create_poll(
        "Test", "Test", "TestLink1234",
        Governance.create_execute_msg(
            community_contract,
            Community.spend(deployer.key.acc_address, spend_amt)
        )
    )

    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            gov_contract, DEFAULT_PROPOSAL_DEPOSIT, poll
        ),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    # Stake
    print(
        f"[deploy] - stake 50% of basket tokens"
    )

    stake_amount = "800000000000"
    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            gov_contract, stake_amount, Governance.stake_voting_tokens()
        ),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    # cast vote
    print(
        f"[deploy] - cast vote for YES"
    )

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.cast_vote(3, "yes", stake_amount),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )
    print(result.logs[0].events_by_type)

    # # increase block time (?)
    # global sequence
    # sequence += DEFAULT_EFFECTIVE_DELAY

    # execute poll
    print(f"sequence # is: {deployer.sequence()}")
    print(
        f"[deploy] - execute vote"
    )

    time.sleep(2)

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.end_poll(3),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    time.sleep(2)

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.execute_poll(3),
        seq(),
        fee=StdFee(
            4000000, "20000000uluna"
        ),
    )

    print(result.logs[0].events_by_type)

    res = result.logs[0].events_by_type
    assert res['from_contract']['recipient'][0] == deployer.key.acc_address
    assert res['from_contract']['amount'][0] == spend_amt