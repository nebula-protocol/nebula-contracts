# Usage guide on Tequila

## Network Settings

For best experience, the following are recommended:

- chain ID: `tequila-0004`
- URL: `https://tequila-lcd.terra.dev`
- gas prices: https://tequila-fcd.terra.dev/v1/txs/gas_prices

```json
{
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
  "usgd": "0.2"
}
```

## Contract Addresses

```jsonc
{
  "wBTC": "terra1l77k5tjgf2mj3z200gz2gyyuqc365tpagdmr3w",
  "wETH": "terra16e0v5jvvrak0zdpwjpn82m5yyqs4fnjuuv940c",
  "wXRP": "terra1nefk50m70k6aeafug92mawnsxs5gcdv8vud4pa",
  "wLUNA": "terra169yde42c4unr2jyjf8yp8z4a7dq95nljz8gce9",
  "MIR": "terra1ygrjh9aj4mq3qx8d3lnehlcqldk5jgn8fhpfva",
  "basketToken": "terra1pmruve9htmsaynwpgefx8dkp2jj7qgaa0qevhg",
  "basket": "terra1pwuwr5eg3tffau24tzpcn2mgm8aw4ndn0e032n",
  "oracle": "terra14d6mkfwa8carsfhyyyka5m7ruvad7643rtajq8"
}
```

## Accounts

### Basket Contract Owner

Is able to invoke permissioned functions on the Basket contract

`terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`

#### Mnemonic

```
notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius
```

### Basket Contract User

Your testing user (will perform main operations against)

`terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v`

#### Mnemonic

```
notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius
```
