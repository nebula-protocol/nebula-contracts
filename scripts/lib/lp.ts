import { Coin, Coins } from "@terra-money/terra.js";
import { newClient, TokenAsset, NativeAsset } from "./helpers.js";

import { execute } from "./tx.js";

export async function createPair(network: any, token: string) {
  const { terra, wallet } = newClient();

  const tokenAddress = network[`${token}TokenAddress`];

  // Create Pair
  let assetInfos = [
    new TokenAsset(tokenAddress).getInfo(),
    new NativeAsset("uusd").getInfo(),
  ];

  let pairCreationTx = await execute(
    "create_pair",
    network.astroportFactoryAddress,
    terra,
    wallet,
    {
      create_pair: {
        pair_type: { xyk: {} },
        asset_infos: assetInfos,
        init_params: null,
      },
    }
  );

  const pairAddressName = `${token}USTPairAddress`;
  const lpAddressName = `${token}USTLPAddress`;

  network[pairAddressName] = JSON.parse(pairCreationTx["raw_log"])[0][
    "events"
  ][1]["attributes"][3]["value"];
  console.log(`${pairAddressName}: ${network[pairAddressName]}`);

  network[lpAddressName] = JSON.parse(pairCreationTx["raw_log"])[0][
    "events"
  ][1]["attributes"][4]["value"];
  console.log(`${lpAddressName}: ${network[lpAddressName]}`);

  return network;
}

export async function provideLiquidity(
  network: any,
  asset1: NativeAsset | TokenAsset,
  asset2: NativeAsset | TokenAsset
) {
  const { terra, wallet } = newClient();

  let assets = [asset1, asset2];
  let coins = [];

  for (const key in assets) {
    const asset = assets[key];

    // send tokens
    if (asset instanceof NativeAsset) {
      coins.push(asset.toCoin());
    }

    // set allowance
    if (asset instanceof TokenAsset) {
      console.log("Setting allowance for contract");
      await execute("increase_allowance", asset.addr, terra, wallet, {
        increase_allowance: {
          spender: network.nebUSTPairAddress,
          amount: asset.amount,
          expires: {
            never: {},
          },
        },
      });
    }
  }

  await execute(
    "provide_liquidity",
    network.nebUSTPairAddress,
    terra,
    wallet,
    {
      provide_liquidity: {
        assets: [asset1.withAmount(), asset2.withAmount()],
      },
    },
    new Coins([new Coin("uusd", "10000000")])
  );
  return network;
}
