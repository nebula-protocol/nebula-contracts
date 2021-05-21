from basket import Oracle, Basket, CW20, Asset, Governance
from contract_helpers import get_terra, get_deployer, store_contract, instantiate_contract, execute_contract, get_amount, seq, get_contract_ids
import time
from terra_sdk.core.auth import StdFee

DEFAULT_PROPOSAL_DEPOSIT = "10000000000"


def create_cluster_through_governance(penalty_contract, factory_contract, oracle, wBTC, wETH, deployer, nebula_token, gov_contract, factory_code_id, basket_code_id, token_code_id, pair_code_id):
    gov_create_basket_msg = {
        "create_cluster": {
            "name": "GOVBASKET",
            "symbol": "GVB",
            "params": {
                "name": "GOVBASKET",
                "symbol": "GVB",
                "penalty": penalty_contract,
                "target": [50, 50],
                "assets": [
                    Asset.cw20_asset_info(wBTC),
                    Asset.cw20_asset_info(wETH),
                ],
                "oracle": oracle,
            },
        }
    }

    poll_two = Governance.create_poll(
        "Test_two",
        "Test_two",
        "TestLink12345",
        Governance.create_execute_msg(
            factory_contract, gov_create_basket_msg
        ),
    )

    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(gov_contract, DEFAULT_PROPOSAL_DEPOSIT, poll_two),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )
    print(result.logs[0].events_by_type)

        # Stake
    print(f"[deploy] - stake 50% of nebula tokens")

    stake_amount = "500000000000"
    result = execute_contract(
        deployer,
        nebula_token,
        CW20.send(gov_contract, stake_amount, Governance.stake_voting_tokens()),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    print(result.logs[0].events_by_type)

    print(f"[deploy] - cast vote for YES")

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.cast_vote(1, "yes", stake_amount),
        seq(),
        fee=StdFee(4000000, "200000uluna"),
    )

    # ensure poll results are reflected on chain
    time.sleep(5)

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.end_poll(1),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    result = execute_contract(
        deployer,
        gov_contract,
        Governance.execute_poll(1),
        seq(),
        fee=StdFee(4000000, "20000000uluna"),
    )

    logs = result.logs[0].events_by_type
    instantiation_logs = logs["instantiate_contract"]
    code_ids = instantiation_logs["code_id"]
    addresses = instantiation_logs["contract_address"]
    assert code_ids[3] == basket_code_id
    basket = addresses[3]
    assert code_ids[2] == token_code_id
    basket_token = addresses[2]
    assert code_ids[1] == pair_code_id
    pair_contract = addresses[1]
    assert code_ids[0] == token_code_id
    lp_token = addresses[0]

    print(f"[deploy] - basket {basket}, basket_token {basket_token}, pair_contract {pair_contract}, lp_token {lp_token}")
    # print(basket, basket_token, pair_contract, lp_token)

    print("Cluster created through governance")
    return basket, basket_token, pair_contract, lp_token