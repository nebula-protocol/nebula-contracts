"""Test governance deploy script.

NOTE: Normally, we can use fee estimation in Tequila, as well as rely on Wallet to auto
fetch the sequence number from the blockchain. Here, we have manual options for sequence
number and fee.

Why manually incrementing sequence number: tequila endpoint is load-balanced so in successive
transactions, the nodes may not have had time to catch up to each other, which may result
in a signature (chain id, account, sequence) mismatch.

Why manually setting fee: tequila node allows simulating (auto-estimating) fee up to
3000000 gas. Some transactions such as code uploads and burning basket token (which
incurs multiple CW20 transfers to the user may require more gas than permitted by the
fee estimation feature).
"""

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
from instantiate import *
from create_cluster import create_cluster_through_governance
from basket_interactions import basket_operations
from governance_penalty_ops import create_new_penalty_with_gov
from liquidity_providing_ops import lp_staking_queries
from community_ops import community_operations
from airdrop_ops import airdrop_operation

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

terra = get_terra()
deployer = get_deployer()

sequence = deployer.sequence()


def deploy():

    print(f"DEPLOYING WITH ACCCOUNT: {deployer.key.acc_address}")
    token_code_id, oracle_code_id, basket_code_id, penalty_code_id, terraswap_factory_code_id, pair_code_id, staking_code_id, collector_code_id, gov_code_id, factory_code_id, community_id, airdrop_id, incentives_id = get_contract_ids()

    terraswap_factory_contract = instantiate_terraswap_factory_contract(terraswap_factory_code_id, pair_code_id, token_code_id)

    factory_contract = instantiate_factory_contract(factory_code_id, token_code_id, basket_code_id)

    nebula_token = instantiate_nebula_token(token_code_id, factory_contract)

    incentives_contract = instantiate_incentives_contract(
        incentives_id,
        factory_contract,
        terraswap_factory_contract,
        nebula_token
    )

    print(f"[deploy] - create terraswap pair from factory for neb token")
    resp = execute_contract(
        deployer,
        terraswap_factory_contract,
        {
            "create_pair": {
                "asset_infos": [
                    {"token": {"contract_addr": nebula_token}},
                    {"native_token": {"denom": "uusd"}},
                ]
            }
        },
        seq(),
    )

    log = resp.logs[0].events_by_type
    neb_pair_contract = log["from_contract"]["pair_contract_addr"][0]

    print(f"[deploy] - adding liquidity to nebula pair contract")
    execute_contract(
        deployer,
        nebula_token,
        {"increase_allowance": {"spender": neb_pair_contract, "amount": "100"}},
        seq(),
    )

    staking_contract = instantiate_staking_contract(staking_code_id, factory_contract, nebula_token, terraswap_factory_contract)

    # instantiate nebula governance contract
    gov_contract = instantiate_gov_contract(gov_code_id, nebula_token)

    community_contract = instantiate_community_contract(community_id, gov_contract, nebula_token)

    airdrop_contract = instantiate_airdrop_contract(airdrop_id, nebula_token)

    collector_contract = instantiate_collector_contract(collector_code_id, gov_contract, terraswap_factory_contract, nebula_token, factory_contract)

    print(f"[deploy] - post initialize factory")
    resp = execute_contract(
        deployer,
        factory_contract,
        {
            "post_initialize": {
                "owner": gov_contract,
                "nebula_token": nebula_token,
                "oracle_contract": nebula_token,  # ??? provide arbitrary contract for now
                "terraswap_factory": terraswap_factory_contract,
                "staking_contract": staking_contract,
                "commission_collector": collector_contract,
            }
        },
        seq(),
    )

    penalty_contract = instantiate_penalty_contract(penalty_code_id, factory_contract)

    # wrapped btc
    wBTC = instantiate_wbtc_contract(token_code_id)

    # wrapped ether
    wETH = instantiate_weth_contract(token_code_id)

    # instantiate oracle
    oracle = instantiate_oracle_contract(oracle_code_id)

    stage_and_mint_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address,
                neb_pair_contract,
                {
                    "provide_liquidity": {
                        "assets": [
                            {
                                "info": {"token": {"contract_addr": nebula_token}},
                                "amount": "100",
                            },
                            {
                                "info": {"native_token": {"denom": "uusd"}},
                                "amount": "100",
                            },
                        ]
                    },
                },
                {"uusd": "100"},
            ),
        ],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    resp = terra.tx.broadcast(stage_and_mint_tx)
    if resp.is_tx_error():
        raise Exception(resp.raw_log)


    execute_contract(
        deployer,
        oracle,
        Oracle.set_prices(
            [
                [wBTC, "30000.0"],
                [wETH, "1500.0"],
                ["uluna", "15.00"],
            ]
        ),
        seq(),
    )

    #CREATE BASKET THROUGH GOV VOTING
    basket, basket_token, pair_contract, lp_token = create_cluster_through_governance(penalty_contract, factory_contract, oracle, wBTC, wETH, deployer, nebula_token, gov_contract, factory_code_id, basket_code_id, token_code_id, pair_code_id)

    #TEST BASKET OPS (PROVIDE LIQUIDITY, MINT, REDEEM)
    basket_operations(wBTC, wETH, basket_token, collector_contract, pair_contract, basket, nebula_token)


    # GOVERNANCE VOTING FOR NEB REWARDS
    create_new_penalty_with_gov(nebula_token, gov_contract, penalty_code_id, collector_contract, basket)

    # QUERY BALANCES POST OPERATIONS
    lp_staking_queries(lp_token, staking_contract, basket_token, factory_contract, nebula_token)


    # TEST COMMUNITY VOTING
    community_operations(nebula_token, community_contract, gov_contract)

    # TEST AIRDROP OPERATIONS
    airdrop_operation(nebula_token, airdrop_contract)

    print("PAIR", terra.wasm.contract_query(
        pair_contract,
        {"pool": {}}
    ))

    for _ in range(3):
        arb_redeem_tx = deployer.create_and_sign_tx(
            msgs=[
                MsgExecuteContract(
                    deployer.key.acc_address,
                    incentives_contract,
                    {
                        "arb_cluster_redeem": {
                            "basket_contract": basket,
                            "asset": Asset.asset("uusd", "500", native=True),
                        }
                    },
                    {"uusd": "500"}
                ),
            ],
            sequence=seq(),
            fee=StdFee(4000000, "2000000uluna"),
        )

        print("PAIR", terra.wasm.contract_query(
            pair_contract,
            {"pool": {}}
        ))

        result = terra.tx.broadcast(arb_redeem_tx)
        print(factory_contract, terraswap_factory_contract, deployer.key.acc_address)
        if result.is_tx_error():
            raise Exception(result.raw_log)

        for log in result.logs:
            import pprint
            pprint.pprint(log.events_by_type)

    execute_contract(
        deployer,
        nebula_token,
        CW20.send(
            incentives_contract,
            "1000",
            {"deposit_reward": {"rewards": [[1, basket, "1000"]]}}
        ),
        seq()
    )

    execute_contract(
        deployer,
        incentives_contract,
        {"new_penalty_period": {}},
        seq()
    )

    result = execute_contract(
        deployer,
        incentives_contract,
        {"withdraw": {}},
        seq()
    )

    for log in result.logs:
        import pprint
        pprint.pprint(log.events_by_type)



deploy()
