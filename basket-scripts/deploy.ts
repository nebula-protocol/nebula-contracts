import {
  MsgStoreCode,
  MsgExecuteContract,
  MsgInstantiateContract,
  LocalTerra,
  isTxError,
  Wallet,
  StdFee,
} from "@terra-money/terra.js";

import * as fs from "fs";

const terra = new LocalTerra();
terra.config.gasPrices = {
  uluna: 1,
};
const deployer: Wallet = terra.wallets.test1;

async function storeContract(contractName: string): Promise<string> {
  console.log(`[storeContract] - ${contractName}`);
  const storeCode = new MsgStoreCode(
    deployer.key.accAddress,
    fs.readFileSync(`../artifacts/${contractName}.wasm`).toString("base64")
  );
  const storeCodeTx = await deployer.createAndSignTx({
    msgs: [storeCode],
    fee: new StdFee(5000000, { uluna: 20000000 }),
  });
  const storeCodeTxResult = await terra.tx.broadcast(storeCodeTx);
  console.log(storeCodeTxResult);

  if (isTxError(storeCodeTxResult)) {
    throw new Error(
      `[storeContract] TX FAILED. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}, raw_log: ${storeCodeTxResult.raw_log}`
    );
  }

  const {
    store_code: { code_id },
  } = storeCodeTxResult.logs[0].eventsByType;

  console.log(`[storeContract] - SUCCESS, code id: ${code_id[0]}`);
  return code_id[0];
}

async function instantiateContract(
  codeId: string,
  initMsg: any
): Promise<string> {
  const instantiate = new MsgInstantiateContract(
    deployer.key.accAddress,
    +codeId,
    initMsg
  );

  const instantiateTx = await deployer.createAndSignTx({
    msgs: [instantiate],
    fee: new StdFee(5000000, { uluna: 20000000 }),
  });

  const instantiateTxResult = await terra.tx.broadcast(instantiateTx);

  console.log(instantiateTxResult);

  if (isTxError(instantiateTxResult)) {
    throw new Error(
      `[instantiateContract] TX FAILED - code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}, raw_log: ${instantiateTxResult.raw_log}`
    );
  }

  const {
    instantiate_contract: { contract_address },
  } = instantiateTxResult.logs[0].eventsByType;

  console.log(
    `[instantiateContract] - instantiated contract address: ${contract_address[0]}`
  );
  return contract_address[0];
}

async function instantiateTokenContract(
  tokenCodeId: string,
  name: string,
  symbol: string,
  supply?: string,
  minter?: string
): Promise<string> {
  let initial_balances: Array<{ address: string; amount: string }> = [];
  if (supply) {
    initial_balances = [{ address: deployer.key.accAddress, amount: supply }];
  }
  console.log(`[instantiateTokenContract] token symbol: ${symbol}`);
  const initMsg = {
    name,
    symbol,
    decimals: 6,
    initial_balances,
    mint: minter && {
      minter,
      cap: null,
    },
  };
  console.log(initMsg);
  return instantiateContract(tokenCodeId, initMsg);
}

async function executeMany(reqs: Array<[string, any]>): Promise<any> {
  let msgs = reqs.map(([contractAddress, executeMsg]) => {
    console.log(`[executeMulti] contract address: ${contractAddress}`);
    return new MsgExecuteContract(
      deployer.key.accAddress, // sender
      contractAddress, // contract account address
      executeMsg
    );
  });

  const executeTx = await deployer.createAndSignTx({
    msgs,
    fee: new StdFee(1000000 * msgs.length, { uluna: 20000000 * msgs.length }),
  });

  const res = await terra.tx.broadcast(executeTx);

  console.log(res);
  if (!isTxError(res)) {
    res.logs.forEach((x) =>
      console.log(JSON.stringify(x.eventsByType, null, 2))
    );
  }
  return res;
}

async function executeContract(
  contractAddress: string,
  executeMsg: any
): Promise<any> {
  console.log(`[executeContract] contract address: ${contractAddress}`);
  let msgs = [
    new MsgExecuteContract(
      deployer.key.accAddress, // sender
      contractAddress, // contract account address
      executeMsg
    ),
  ];

  const executeTx = await deployer.createAndSignTx({
    msgs,
    fee: new StdFee(1000000, { uluna: 30000000 }),
  });

  const res = await terra.tx.broadcast(executeTx);
  console.log(res);
  if (!isTxError(res)) {
    res.logs.forEach((x) =>
      console.log(JSON.stringify(x.eventsByType, null, 2))
    );
  }
  return res;
}

async function main() {
  const tokenCodeId = await storeContract("terraswap_token");
  const oracleCodeId = await storeContract("basket_dummy_oracle");
  const basketCodeId = await storeContract("basket_contract");

  // instantiate asset tokens
  const mAAPLAddress = await instantiateTokenContract(
    tokenCodeId,
    "Mirrored Apple",
    "mAAPL",
    "1000000000000"
  );
  const mGOOGAddress = await instantiateTokenContract(
    tokenCodeId,
    "Mirrored Google",
    "mGOOG",
    "1000000000000"
  );
  const mMSFTAddress = await instantiateTokenContract(
    tokenCodeId,
    "Mirrored Microsoft",
    "mMSFT",
    "1000000000000"
  );
  const mNFLXAddress = await instantiateTokenContract(
    tokenCodeId,
    "Mirrored Netflix",
    "mNFLX",
    "1000000000000"
  );

  // instantiate oracle
  const oracleAddress = await instantiateContract(oracleCodeId, {});

  // instantiate basket
  const basketAddress = await instantiateContract(basketCodeId, {
    name: "Basket",
    owner: deployer.key.accAddress,
    assets: [mAAPLAddress, mGOOGAddress, mMSFTAddress, mNFLXAddress],
    oracle: oracleAddress,
    penalty_params: {
      a_pos: "1",
      s_pos: "1",
      a_neg: "0.005",
      s_neg: "0.5",
    },
    target: [10, 20, 65, 5],
  });

  // instantiate basket token
  const basketTokenAddress = await instantiateTokenContract(
    tokenCodeId,
    "Basket Token",
    "BSKT",
    "1000000000",
    basketAddress
  );

  // set basket token
  await executeContract(basketAddress, {
    __set_basket_token: {
      basket_token: basketTokenAddress,
    },
  });

  // set oracle prices
  await executeContract(oracleAddress, {
    set_prices: {
      prices: [
        [mAAPLAddress, "135.18"],
        [mGOOGAddress, "1753.47"],
        [mMSFTAddress, "219.88"],
        [mNFLXAddress, "540.82"],
      ],
    },
  });

  // give basket starting balance
  await executeContract(mAAPLAddress, {
    transfer: {
      recipient: basketAddress,
      amount: "7290053159",
    },
  });

  await executeContract(mGOOGAddress, {
    transfer: {
      recipient: basketAddress,
      amount: "319710128",
    },
  });

  await executeContract(mMSFTAddress, {
    transfer: {
      recipient: basketAddress,
      amount: "1421928123",
    },
  });

  await executeContract(mNFLXAddress, {
    transfer: {
      recipient: basketAddress,
      amount: "224212221",
    },
  });

  // try mint
  console.log("[main] - mint (staging)");
  await executeMany([
    [
      mAAPLAddress,
      {
        send: {
          contract: basketAddress,
          amount: "125000000",
          msg: Buffer.from(
            JSON.stringify({
              stage_asset: {},
            })
          ).toString("base64"),
        },
      },
    ],
    [
      mMSFTAddress,
      {
        send: {
          contract: basketAddress,
          amount: "149000000",
          msg: Buffer.from(
            JSON.stringify({
              stage_asset: {},
            })
          ).toString("base64"),
        },
      },
    ],
    [
      mNFLXAddress,
      {
        send: {
          contract: basketAddress,
          amount: "50000000",
          msg: Buffer.from(
            JSON.stringify({
              stage_asset: {},
            })
          ).toString("base64"),
        },
      },
    ],
  ]);

  console.log("[mint] - mint");
  await executeContract(basketAddress, {
    mint: {
      asset_amounts: ["125000000", "0", "149000000", "50000000"],
    },
  });

  console.log({
    mAAPLAddress,
    mGOOGAddress,
    mMSFTAddress,
    mNFLXAddress,
    basketTokenAddress,
    basketAddress,
  });
}

main().catch(console.error);
