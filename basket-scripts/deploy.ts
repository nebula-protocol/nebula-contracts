import {
  storeContract,
  instantiateTokenContract,
  instantiateContract,
  terra,
  deployer,
  executeContract,
  Oracle,
  CW20,
  Basket,
  executeMany,
} from "./util";

async function main() {
  const tokenCodeId = await storeContract("terraswap_token");
  const oracleCodeId = await storeContract("basket_dummy_oracle");
  const basketCodeId = await storeContract("basket_contract");

  // instantiate asset tokens
  const wBTC = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Bitcoin",
    "wBTC",
    "100000000" // 100 BTC
  );
  const wETH = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Ethereum",
    "wETH",
    "20000000000" // 20000 ETH
  );
  const wXRP = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Ripple",
    "wXRP",
    "5000000000000" // 5m XRP
  );
  const wLUNA = await instantiateTokenContract(
    tokenCodeId,
    "Wrapped Luna",
    "wLUNA",
    "1000000000000" // 1m LUNA
  );
  const MIR = await instantiateTokenContract(
    tokenCodeId,
    "Mirror Token",
    "wMIR",
    "1000000000000" // 1m MIR
  );

  // instantiate oracle
  const oracle = await instantiateContract(oracleCodeId, {});

  // instantiate basket
  const basket = await instantiateContract(basketCodeId, {
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
  });

  // instantiate basket token
  const basketToken = await instantiateTokenContract(
    tokenCodeId,
    "Basket Token",
    "BSKT",
    "1000000000000", // initial starting balance = 1m
    basket
  );

  // set basket token
  await executeContract(basket, Basket.setBasketToken(basketToken));

  // set oracle prices
  await executeContract(
    oracle,
    Oracle.setPrices([
      [wBTC, "30000.0"],
      [wETH, "1500.0"],
      [wXRP, "0.45"],
      [wLUNA, "2.1"],
      [MIR, "5.06"],
    ])
  );

  // give basket starting balance
  await executeContract(wBTC, CW20.transfer(basket, "7290059")); // 8%
  await executeContract(wETH, CW20.transfer(basket, "7290059")); // 18%
  await executeContract(wXRP, CW20.transfer(basket, "23292911")); // 20%
  await executeContract(wLUNA, CW20.transfer(basket, "10102302")); // 32%
  await executeContract(MIR, CW20.transfer(basket, "1010232")); //

  // try mint
  console.log("[main] - basket:stage_asset + basket:mint");
  await executeMany([
    [wBTC, CW20.send(basket, "125000000", Basket.stageAsset())],
    [
      basket,
      Basket.mint(["125000000", "0", "149000000", "50000000", "10202020"]),
    ],
  ]);

  // try burn
  console.log("[main] - basket:burn");
  await executeContract(
    basketToken,
    CW20.send(basket, "10000000", Basket.burn([1, 1, 1, 9, 2]))
  );

  console.log({
    wBTC,
    wETH,
    wXRP,
    wLUNA,
    MIR,
    basketToken,
    basket,
  });
}

main().catch(console.error);
