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
from governance_helpers import create_and_vote_poll
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

    poll_msg = Governance.create_execute_msg(
        community_contract,
        Community.spend(deployer.key.acc_address, spend_amt)
    )
    result = create_and_vote_poll(deployer, nebula_token, poll_msg, gov_contract)

    print(result.logs[0].events_by_type)

    res = result.logs[0].events_by_type
    assert res['from_contract']['recipient'][0] == deployer.key.acc_address
    assert res['from_contract']['amount'][0] == spend_amt