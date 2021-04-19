from requests import Request, Session
from requests.exceptions import ConnectionError, Timeout, TooManyRedirects
import json

def get_prices(symbols):
  currency = 'USD'
  url = 'https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest'
  parameters = {
    'convert': currency,
    'symbol': ','.join(symbols)
  }
  headers = {
    'Accepts': 'application/json',
    'X-CMC_PRO_API_KEY': 'a7dec7fa-c4b0-4eed-8423-fe5604ba079d',
  }

  session = Session()
  session.headers.update(headers)

  try:
    response = session.get(url, params=parameters)
    data = json.loads(response.text)
    symbol_to_prices = {}
    for symbol in symbols:
      price = data['data'][symbol]['quote'][currency]['price']
      market_cap = data['data'][symbol]['quote'][currency]['market_cap']
      symbol_to_prices[symbol] = price
    return symbol_to_prices
    # print("For {}, price = {}, market_cap = {}".format(symbol, price, market_cap))
  except (ConnectionError, Timeout, TooManyRedirects) as e:
    print(e)
