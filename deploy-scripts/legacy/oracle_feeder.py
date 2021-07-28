from requests import Request, Session
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects
import json
import sys
from contract_helpers import Contract
import asyncio


def get_prices(symbols):
    currency = "USD"
    url = "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest"
    sys = [a[0] for a in symbols]
    parameters = {"convert": currency, "symbol": ",".join(sys)}

    headers = {
        "Accepts": "application/json",
        "X-CMC_PRO_API_KEY": "a7dec7fa-c4b0-4eed-8423-fe5604ba079d",
    }

    session = Session()
    session.headers.update(headers)

    try:
        response = session.get(url, params=parameters)
        data = json.loads(response.text)
        symbol_to_prices = {}
        for s in symbols:
            symbol, name = s
            price = data["data"][symbol]["quote"][currency]["price"]
            print(symbol, price)
            market_cap = data["data"][symbol]["quote"][currency]["market_cap"]
            symbol_to_prices[symbol] = {
                "symbol": symbol,
                "name": name,
                "price": price,
                "market_cap": market_cap,
            }
        return symbol_to_prices
        # print("For {}, price = {}, market_cap = {}".format(symbol, price, market_cap))
    except (ConnectionError, Timeout, TooManyRedirects) as e:
        print(e)


basket_addr = sys.argv[1]
basket = Contract(basket_addr)


async def main():

    cfg = (await basket.query.config())["config"]
    oracle = Contract(cfg["pricing_oracle"])

    basket_state = await basket.query.basket_state(basket_contract_address=basket_addr)

    contract_addrs = []
    symbols = []

    for asset in basket_state["assets"]:
        addr = asset["token"]["contract_addr"]
        token_info = await Contract(addr).query.token_info()
        contract_addrs.append(addr)
        symbols.append([token_info["symbol"], token_info["name"]])

    while True:
        price_data = get_prices(symbols)
        set_prices_data = []
        for i in range(len(contract_addrs)):
            set_prices_data.append(
                [contract_addrs[i], str(price_data[symbols[i][0]]["price"])]
            )

        await oracle.set_prices(prices=set_prices_data)
        basket_state = await basket.query.basket_state(
            basket_contract_address=basket_addr
        )
        print("new prices", price_data)
        await asyncio.sleep(30)


if __name__ == "__main__":
    asyncio.get_event_loop().run_until_complete(main())
