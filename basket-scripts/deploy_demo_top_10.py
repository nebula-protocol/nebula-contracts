"""Sample deploy script.

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

from basket import Oracle, Basket, CW20, Asset
from oracle_feeder import get_prices, get_top_15_market_cap
from requests import Request, Session
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects
import json

# If True, use localterra. Otherwise, deploys on Tequila
USE_LOCALTERRA = True

HARD_DATA = [
    [
        {'symbol': 'BTC', 'name': 'Bitcoin', 'market_cap': 1060123509761.5577, 'price': 56733.93241461603}, 
        {'symbol': 'ETH', 'name': 'Ethereum', 'market_cap': 258241284658.2259, 'price': 2235.3549701988345}, 
        {'symbol': 'BNB', 'name': 'Binance Coin', 'market_cap': 77604093031.36272, 'price': 505.7852295610551}, 
        {'symbol': 'XRP', 'name': 'XRP', 'market_cap': 64233759627.76987, 'price': 1.41471498348896}, 
        {'symbol': 'USDT', 'name': 'Tether', 'market_cap': 48143460252.49616, 'price': 0.9999371385729}, 
        {'symbol': 'DOGE', 'name': 'Dogecoin', 'market_cap': 42169672926.89015, 'price': 0.32629475972721}, 
        {'symbol': 'ADA', 'name': 'Cardano', 'market_cap': 41502086056.318726, 'price': 1.29903856519512}, 
        {'symbol': 'DOT', 'name': 'Polkadot', 'market_cap': 35344690849.73061, 'price': 37.9643961562771}, 
        {'symbol': 'BCH', 'name': 'Bitcoin Cash', 'market_cap': 18959657250.55402, 'price': 1013.2027217251489}, 
        {'symbol': 'LTC', 'name': 'Litecoin', 'market_cap': 18398192426.94358, 'price': 275.618381155374}, 
        {'symbol': 'LINK', 'name': 'Chainlink', 'market_cap': 17124692714.985268, 'price': 40.86945620216298}, 
        {'symbol': 'VET', 'name': 'VeChain', 'market_cap': 16604040717.068766, 'price': 0.25816515212028}, 
        {'symbol': 'UNI', 'name': 'Uniswap', 'market_cap': 16600639748.245836, 'price': 31.71780842984127}, 
        {'symbol': 'XLM', 'name': 'Stellar', 'market_cap': 12575828504.56768, 'price': 0.54960038207931}, 
        {'symbol': 'THETA', 'name': 'THETA', 'market_cap': 11934890200.179289, 'price': 11.93489020017929}
    ], 
    [
        {'symbol': 'BTC', 'name': 'Bitcoin', 'market_cap': 1060123509761.5577, 'price': 56733.93241461603}, 
        {'symbol': 'ETH', 'name': 'Ethereum', 'market_cap': 258241284658.2259, 'price': 2235.3549701988345}, 
        {'symbol': 'BNB', 'name': 'Binance Coin', 'market_cap': 77604093031.36272, 'price': 505.7852295610551}, 
        {'symbol': 'XRP', 'name': 'XRP', 'market_cap': 64233759627.76987, 'price': 1.41471498348896}, 
        {'symbol': 'USDT', 'name': 'Tether', 'market_cap': 48143460252.49616, 'price': 0.9999371385729}, 
        {'symbol': 'DOGE', 'name': 'Dogecoin', 'market_cap': 42169672926.89015, 'price': 0.32629475972721}, 
        {'symbol': 'ADA', 'name': 'Cardano', 'market_cap': 41502086056.318726, 'price': 1.29903856519512}, 
        {'symbol': 'DOT', 'name': 'Polkadot', 'market_cap': 35344690849.73061, 'price': 37.9643961562771}, 
        {'symbol': 'BCH', 'name': 'Bitcoin Cash', 'market_cap': 18959657250.55402, 'price': 1013.2027217251489}, 
        {'symbol': 'LINK', 'name': 'Chainlink', 'market_cap': 18458192426.985268, 'price': 40.86945620216298}, 
        {'symbol': 'LTC', 'name': 'Litecoin', 'market_cap': 18398192426.94358, 'price': 275.618381155374}, 
        {'symbol': 'VET', 'name': 'VeChain', 'market_cap': 16604040717.068766, 'price': 0.25816515212028}, 
        {'symbol': 'UNI', 'name': 'Uniswap', 'market_cap': 16600639748.245836, 'price': 31.71780842984127}, 
        {'symbol': 'XLM', 'name': 'Stellar', 'market_cap': 12575828504.56768, 'price': 0.54960038207931}, 
        {'symbol': 'THETA', 'name': 'THETA', 'market_cap': 11934890200.179289, 'price': 11.93489020017929}
    ], 
    [
        {'symbol': 'BTC', 'name': 'Bitcoin', 'market_cap': 760123509761.5577, 'price': 45733.93241461603}, 
        {'symbol': 'ETH', 'name': 'Ethereum', 'market_cap': 208241284658.2259, 'price': 1535.3549701988345}, 
        {'symbol': 'BNB', 'name': 'Binance Coin', 'market_cap': 50604093031.36272, 'price': 305.7852295610551}, 
        {'symbol': 'XRP', 'name': 'XRP', 'market_cap': 44233759627.76987, 'price': 0.91471498348896}, 
        {'symbol': 'USDT', 'name': 'Tether', 'market_cap': 38143460252.49616, 'price': 0.9999371385729}, 
        {'symbol': 'DOGE', 'name': 'Dogecoin', 'market_cap': 42169672926.89015, 'price': 0.032629475972721}, 
        {'symbol': 'ADA', 'name': 'Cardano', 'market_cap': 41502086056.318726, 'price': 1.29903856519512}, 
        {'symbol': 'DOT', 'name': 'Polkadot', 'market_cap': 35344690849.73061, 'price': 37.9643961562771}, 
        {'symbol': 'BCH', 'name': 'Bitcoin Cash', 'market_cap': 18959657250.55402, 'price': 1013.2027217251489}, 
        {'symbol': 'LINK', 'name': 'Chainlink', 'market_cap': 18458192426.985268, 'price': 20.86945620216298}, 
        {'symbol': 'LTC', 'name': 'Litecoin', 'market_cap': 18398192426.94358, 'price': 13.618381155374}, 
        {'symbol': 'VET', 'name': 'VeChain', 'market_cap': 16604040717.068766, 'price': 0.25816515212028}, 
        {'symbol': 'UNI', 'name': 'Uniswap', 'market_cap': 16600639748.245836, 'price': 31.71780842984127}, 
        {'symbol': 'XLM', 'name': 'Stellar', 'market_cap': 12575828504.56768, 'price': 0.54960038207931}, 
        {'symbol': 'THETA', 'name': 'THETA', 'market_cap': 11934890200.179289, 'price': 11.93489020017929}
    ], 
]

lt = LocalTerra(gas_prices = {
        "uluna": "0.15"
    })

if USE_LOCALTERRA:
    terra = lt
    deployer = lt.wallets["test1"]
else:
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

    terra = LCDClient(
        "https://tequila-fcd.terra.dev", "tequila-0004", gas_prices=gas_prices
    )
    deployer = terra.wallet(lt.wallets["test1"].key)

def store_contract(contract_name, sequence):
    contract_bytes = read_file_as_b64(f"../artifacts/{contract_name}.wasm")
    store_code = MsgStoreCode(deployer.key.acc_address, contract_bytes)
    store_code_tx = deployer.create_and_sign_tx(
        msgs=[store_code], fee=StdFee(5000000, "2000000uluna"), sequence=sequence
    )
    result = terra.tx.broadcast(store_code_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_code_id(result)


def instantiate_contract(code_id, init_msg, sequence):
    instantiate = MsgInstantiateContract(deployer.key.acc_address, code_id, init_msg)
    instantiate_tx = deployer.create_and_sign_tx(
        msgs=[instantiate], sequence=sequence, fee_denoms=["uluna"]
    )
    result = terra.tx.broadcast(instantiate_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return get_contract_address(result)


def execute_contract(wallet, contract_address, execute_msg, sequence, fee=None):
    execute = MsgExecuteContract(wallet.key.acc_address, contract_address, execute_msg)
    execute_tx = wallet.create_and_sign_tx(
        msgs=[execute], sequence=sequence, fee_denoms=["uluna"], fee=fee
    )
    result = terra.tx.broadcast(execute_tx)
    if result.is_tx_error():
        print(result.raw_log)
    return result


def get_amount(value, price):
    """Gets Uint128 amount of coin in order to get total value, assuming price."""
    return str(int(value / float(price) * 1000000))


sequence = deployer.sequence()


def seq():
    """Increments global sequence."""
    global sequence
    sequence += 1
    return sequence - 1


def deploy():
    print(f"DEPLOYING WITH ACCCOUNT: {deployer.key.acc_address}")
    print(f"[deploy] - store terraswap_token")
    token_code_id = store_contract("terraswap_token", seq())

    print(f"[deploy] - store basket_dummy_oracle")
    oracle_code_id = store_contract("basket_dummy_oracle", seq())

    print(f"[deploy] - store basket_contract")
    basket_code_id = store_contract("basket_contract", seq())

    INITIAL_TOKEN_INFO = HARD_DATA[0]

    # "symbol", "name", "market_cap", "price"

    # WBTC -> contract
    assets = []
    for token_info in INITIAL_TOKEN_INFO:
        # wrapped bitcoin
        print(f"[deploy] - instantiate {token_info['name']} ({token_info['symbol']}) at ${token_info['price']}")
        print(token_info)
        contract = instantiate_contract(
                token_code_id,
                {
                    "name": token_info['name'],
                    "symbol": token_info['symbol'],
                    "decimals": 6,
                    "initial_balances": [
                        {"address": deployer.key.acc_address,
                        "amount": token_info['price'] * 10**6 * 400}
                    ],
                    "mint": None,
                },
                seq(),
            )
        asset = {
            'contract': contract,
            'symbol': token_info['symbol'],
            'name': token_info['name'],
            'price': token_info['price'],
            'market_cap': token_info['market_cap'],
        }
        assets.append(asset)

    # instantiate oracle
    print(f"[deploy] - instantiate oracle")
    oracle = instantiate_contract(oracle_code_id, {}, seq())
    
    init_top_10 = assets[:10]

    # instantiate basket with top 10
    print(f"[deploy] - instantiate basket with top 10 by market cap (simple average)")
    basket = instantiate_contract(
        basket_code_id,
        {
            "name": "Basket",
            "owner": deployer.key.acc_address,
            "assets": [asset['contract'] for asset in init_top_10],
            "oracle": oracle,
            "penalty_params": {
                "a_pos": "1",
                "s_pos": "1",
                "a_neg": "0.005",
                "s_neg": "0.5",
            },
            "target": [10] * 10,
        },
        seq(),
    )

    # instantiate basket token
    print(f"[deploy] - instantiate basket token")
    basket_token = instantiate_contract(
        token_code_id,
        {
            "name": "Basket Token",
            "symbol": "TOP_TEN_MARKET_CAP",
            "decimals": 6,
            "initial_balances": [
                {"address": deployer.key.acc_address, "amount": "1000000000000"}
            ],
            "mint": {"minter": basket, "cap": None},
        },
        seq(),
    )

    # set basket token
    print(f"[deploy] - set basket token")
    execute_contract(deployer, basket, Basket.set_basket_token(basket_token), seq())

    # sets initial balance of basket contract
    total = 5000000
    initialization_amounts = [get_amount(total * 0.1, str(init_token['price'])) for init_token in init_top_10]
    print("[deploy] - give initial balances")
    for idx, init_token in enumerate(init_top_10):
        print(f"{init_token['symbol']}: {initialization_amounts[idx]}")

    initial_balances_tx = deployer.create_and_sign_tx(
        msgs=[
            MsgExecuteContract(
                deployer.key.acc_address, init_token['contract'], CW20.transfer(basket, initialization_amounts[idx])
            ) for idx, init_token in enumerate(init_top_10)],
        sequence=seq(),
        fee=StdFee(4000000, "2000000uluna"),
    )

    result = terra.tx.broadcast(initial_balances_tx)

    for steps, data in enumerate(HARD_DATA):
        # Update assets with new data
        assets = []
        for token_info in data:
            # wrapped bitcoin
            print(f"[deploy] - instantiate {token_info['name']} ({token_info['symbol']}) at ${token_info['price']}")
            asset = {
                'contract': instantiate_contract(
                    token_code_id,
                    {
                        "name": token_info['name'],
                        "symbol": token_info['symbol'],
                        "decimals": 6,
                        "initial_balances": [
                            {"address": deployer.key.acc_address,
                            "amount": token_info['price'] * 10**6 * 400}
                        ],
                        "mint": None,
                    },
                    seq(),
                ),
                'symbol': token_info['symbol'],
                'name': token_info['name'],
                'price': token_info['price'],
                'market_cap': token_info['market_cap'],
            }
            assets.append(asset)

            prices = [[asset['contract'], asset['price']] for asset in assets]

            # set oracle prices
            print(f"[deploy] - set oracle prices")
            execute_contract(
                deployer,
                oracle,
                Oracle.set_prices(prices),
                seq(),
            )

            # Get top 10
            TOP_10 = assets[:10]

            ### EXAMPLE: how to query basket state

            # GET CURRENT COMPOSITION FROM BASKET STATE
            basket_state = terra.wasm.contract_query(
                    basket, {"basket_state": {"basket_contract_address": basket}}
                )
            print("query basket state ",
                basket_state
            )

            print(TOP_10)


            # COMPARE TO TOP 10 CURRENT ASSETS

            # IF NECESSARY, RESET COMPOSITION

            # IF STEPS = 2, BURN LTC + SHOW REWARD

    #         ### EXAMPLE: how to reset basket composition
    #         print("[deploy] - basket: reset_target")
    #         result = execute_contract(
    #             deployer,
    #             basket,
    #             Basket.reset_target(Asset.asset_info_from_haddrs([wBTC, wETH, wXRP, wLUNA, MIR, ANC]), [10, 20, 15, 20, 20, 15]),
    #             seq(),
    #             fee=StdFee(
    #                 4000000, "20000000uluna"
    #             ),  # burning may require a lot of gas if there are a lot of assets
    #         )
    #         print(f"reset contract TXHASH: {result.txhash}")

    #         ### EXAMPLE: getting event logs
    #         print("logs from reset target", result.logs[0].events_by_type)

    #         print("query new basket state ",
    #             terra.wasm.contract_query(
    #                 basket, {"basket_state": {"basket_contract_address": basket}}
    #             )
    #         )

    # ### EXAMPLE: how to stage and mint

    # print("[deploy] - basket:stage_asset + basket:mint")
    # stage_and_mint_tx = deployer.create_and_sign_tx(
    #     msgs=[
    #         MsgExecuteContract(
    #             deployer.key.acc_address,
    #             wBTC,
    #             CW20.send(basket, "1000000", Basket.stage_asset()),
    #         ),
    #         MsgExecuteContract(
    #             deployer.key.acc_address,
    #             wLUNA,
    #             CW20.send(basket, "4000000000", Basket.stage_asset()),
    #         ),
    #         MsgExecuteContract(
    #             deployer.key.acc_address,
    #             basket,
    #             Basket.mint(
    #                 [Asset.asset(wBTC, "1000000"),
    #                 Asset.asset(wETH, "0"),
    #                 Asset.asset(wXRP, "0"),
    #                 Asset.asset(wLUNA, "4000000000"),
    #                 Asset.asset(MIR, "0")]
    #             ),
    #         ),
    #     ],
    #     sequence=seq(),
    #     fee=StdFee(4000000, "2000000uluna"),
    # )

    # result = terra.tx.broadcast(stage_and_mint_tx)
    # print(f"stage & mint TXHASH: {result.txhash}")


    # ### EXAMPLE: how to burn
    # print("[deploy] - basket:burn")
    # result = execute_contract(
    #     deployer,
    #     basket_token,
    #     CW20.send(
    #         basket, "10000000000", Basket.burn(
    #                 [Asset.asset(wBTC, "1"),
    #                 Asset.asset(wETH, "2"),
    #                 Asset.asset(wXRP, "1"),
    #                 Asset.asset(wLUNA, "0"),
    #                 Asset.asset(MIR, "7")])
    #     ),  # asset weights must be integers
    #     seq(),
    #     fee=StdFee(
    #         4000000, "20000000uluna"
    #     ),  # burning may require a lot of gas if there are a lot of assets
    # )
    # print(f"burn TXHASH: {result.txhash}")

    # ### EXAMPLE: getting event logs
    # print(result.logs[0].events_by_type)

    # print(
    #     terra.wasm.contract_query(
    #         basket_token, {"balance": {"address": deployer.key.acc_address}}
    #     )
    # )

    

    # while True:
    #     tokens = ['WBTC', 'WETH', 'XRP', 'LUNA', 'MIR']
    #     prices = get_prices(tokens)
    #     tot = sum(prices)
    #     percentages = [float(p)/tot for p in prices]
    #     print(percentages)
    #     for i in range(len(percentages) - 1):
    #         percentages[i] = int(percentages[i])
    #     percentages[-1] = 100 - sum(percentages[:-1])
    #     print(percentages, sum(percentages))
    #     time.sleep(30)



deploy()