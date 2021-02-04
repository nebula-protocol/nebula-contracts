import {
  storeContract,
  instantiateTokenContract,
  instantiateContract,
  deployer,
  executeContract,
  Oracle,
  CW20,
  Basket,
  executeMany,
  getAmount,
} from "./util";

async function main() {
  let sequence = await deployer.sequence();
  const tokenCodeId = await storeContract("terraswap_token", sequence++);
  const oracleCodeId = await storeContract("basket_dummy_oracle", sequence++);
  const basketCodeId = await storeContract("basket_contract", sequence++);

  // instantiate asset tokens
  const wBTC = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Bitcoin",
    "wBTC",
    "400000000", // 400 BTC
    undefined,
    sequence++
  );
  const wETH = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Ethereum",
    "wETH",
    "20000000000", // 20000 ETH
    undefined,
    sequence++
  );
  const wXRP = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Ripple",
    "wXRP",
    "5000000000000", // 5m XRP
    undefined,
    sequence++
  );
  const wLUNA = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Luna",
    "wLUNA",
    "1000000000000", // 1m LUNA
    undefined,
    sequence++
  );
  const MIR = await instantiateTokenContract(
    tokenCodeId,
    "Mirror Token",
    "wMIR",
    "1000000000000", // 1m MIR
    undefined,
    sequence++
  );

  // instantiate oracle
  const oracle = await instantiateContract(oracleCodeId, {}, sequence++);

  // instantiate basket
  const basket = await instantiateContract(
    basketCodeId,
    {
      name: "Basket",
      owner: deployer.key.accAddress,
      assets: [wBTC, wETH, wXRP, wLUNA, MIR],
      oracle: oracle,
      penalty_params: {
        a_pos: "1",
        s_pos: "1",
        a_neg: "0.005",
        s_neg: "0.5",
      },
      target: [10, 20, 15, 30, 25],
    },
    sequence++
  );

  // instantiate basket token
  const basketToken = await instantiateTokenContract(
    tokenCodeId,
    "Basket Token",
    "BSKT",
    "1000000000000", // initial starting balance = 1m
    basket,
    sequence++
  );

  // set basket token
  await executeContract(basket, Basket.setBasketToken(basketToken), sequence++);

  // set oracle prices
  await executeContract(
    oracle,
    Oracle.setPrices([
      [wBTC, "30000.0"],
      [wETH, "1500.0"],
      [wXRP, "0.45"],
      [wLUNA, "2.1"],
      [MIR, "5.06"],
    ]),
    sequence++
  );

  // give basket starting balance w/ 5m in notional value
  const total = 5000000;
  await executeContract(
    wBTC,
    CW20.transfer(basket, getAmount(total * 0.08, "30000.0")),
    sequence++
  ); // 8%
  await executeContract(
    wETH,
    CW20.transfer(basket, getAmount(total * 0.18, "1500.0")),
    sequence++
  ); // 18%
  await executeContract(
    wXRP,
    CW20.transfer(basket, getAmount(total * 0.2, "0.45")),
    sequence++
  ); // 20%
  await executeContract(
    wLUNA,
    CW20.transfer(basket, getAmount(total * 0.32, "2.1")),
    sequence++
  ); // 32%
  await executeContract(
    MIR,
    CW20.transfer(basket, getAmount(total * 0.22, "5.06")),
    sequence++
  ); // 22%

  // try mint
  console.log("[main] - basket:stage_asset + basket:mint");
  await executeMany(
    [
      [wBTC, CW20.send(basket, "1000000", Basket.stageAsset())],
      [basket, Basket.mint(["1000000", "0", "0", "0", "0"])],
    ],
    sequence++
  );

  // try burn
  console.log("[main] - basket:burn");
  await executeContract(
    basketToken,
    CW20.send(basket, "10000000", Basket.burn([1, 1, 1, 9, 2])),
    sequence++
  );

  console.log({
    wBTC,
    wETH,
    wXRP,
    wLUNA,
    MIR,
    basketToken,
    basket,
    oracle,
  });
}

main().catch(console.error);
