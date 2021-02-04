import {
  MsgStoreCode,
  MsgExecuteContract,
  MsgInstantiateContract,
  LocalTerra,
  isTxError,
  Wallet,
  StdFee,
  LCDClient,
} from "@terra-money/terra.js";

import * as fs from "fs";

// UNCOMMENT FOR TEQUILA
const lt = new LocalTerra();
export const terra = new LCDClient({
  URL: "https://tequila-lcd.terra.dev",
  chainID: "tequila-0004",
});
export const deployer: Wallet = terra.wallet(lt.wallets.test1.key);

// UNCOMMENT FOR LOCALTERRA
// export const terra = new LocalTerra();
// export const deployer: Wallet = terra.wallets.test1;

export async function storeContract(
  contractName: string,
  sequence?: number
): Promise<string> {
  console.log(`[storeContract] - ${contractName}`);
  const storeCode = new MsgStoreCode(
    deployer.key.accAddress,
    fs.readFileSync(`../artifacts/${contractName}.wasm`).toString("base64")
  );
  const storeCodeTx = await deployer.createAndSignTx({
    msgs: [storeCode],
    fee: new StdFee(5000000, { uluna: 2000000 }),
    sequence,
  });
  const res = await terra.tx.broadcast(storeCodeTx);
  console.log(`[storeContract] - TX Hash: ${res.txhash}`);

  if (isTxError(res)) {
    throw new Error(
      `[storeContract] TX FAILED. code: ${res.code}, codespace: ${res.codespace}, raw_log: ${res.raw_log}`
    );
  }

  const {
    store_code: { code_id },
  } = res.logs[0].eventsByType;

  console.log(`[storeContract] - SUCCESS, code id: ${code_id[0]}`);
  return code_id[0];
}

export async function instantiateContract(
  codeId: string,
  initMsg: any,
  sequence?: number
): Promise<string> {
  const instantiate = new MsgInstantiateContract(
    deployer.key.accAddress,
    +codeId,
    initMsg
  );

  const instantiateTx = await deployer.createAndSignTx({
    msgs: [instantiate],
    fee: new StdFee(5000000, { uluna: 2000000 }),
    sequence,
  });

  const res = await terra.tx.broadcast(instantiateTx);

  console.log(`[instantiateContract] - TX HASH: ${res.txhash}`);

  if (isTxError(res)) {
    throw new Error(
      `[instantiateContract] TX FAILED - code: ${res.code}, codespace: ${res.codespace}, raw_log: ${res.raw_log}`
    );
  }

  const {
    instantiate_contract: { contract_address },
  } = res.logs[0].eventsByType;

  console.log(
    `[instantiateContract] - instantiated contract address: ${contract_address[0]}`
  );
  return contract_address[0];
}

export async function instantiateTokenContract(
  tokenCodeId: string,
  name: string,
  symbol: string,
  supply?: string,
  minter?: string,
  sequence?: number
): Promise<string> {
  let initial_balances: Array<{ address: string; amount: string }> = [];
  if (supply) {
    initial_balances = [{ address: deployer.key.accAddress, amount: supply }];
  }
  console.log(`[instantiateTokenContract] - token symbol: ${symbol}`);
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
  return instantiateContract(tokenCodeId, initMsg, sequence);
}

export function getAmount(value: number, price: string): string {
  return ((value / Number.parseFloat(price)) * 1000000).toFixed();
}

export async function executeMany(
  reqs: Array<[string, any]>,
  sequence?: number
): Promise<any> {
  let msgs = reqs.map(([contractAddress, executeMsg]) => {
    console.log(`[executeMany] - contract address: ${contractAddress}`);
    return new MsgExecuteContract(
      deployer.key.accAddress, // sender
      contractAddress, // contract account address
      executeMsg
    );
  });

  const executeTx = await deployer.createAndSignTx({
    msgs,
    fee: new StdFee(500000 * msgs.length, { uluna: 200000 * msgs.length }),
    sequence,
  });

  const res = await terra.tx.broadcast(executeTx);
  console.log(`[executeMany] - TX Hash: ${res.txhash}`);
  console.log(JSON.stringify(res, null, 2));
  if (!isTxError(res)) {
    res.logs.forEach((x) =>
      console.log(JSON.stringify(x.eventsByType, null, 2))
    );
  }
  return res;
}

export async function executeContract(
  contractAddress: string,
  executeMsg: any,
  sequence?: number
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
    fee: new StdFee(2000000, { uluna: 3000000 }),
    sequence,
  });

  const res = await terra.tx.broadcast(executeTx);
  console.log(`[executeContract] TX Hash: ${res.txhash}`);
  console.log(JSON.stringify(res, null, 2));
  if (!isTxError(res)) {
    res.logs.forEach((x) =>
      console.log(JSON.stringify(x.eventsByType, null, 2))
    );
  }
  return res;
}

export namespace Oracle {
  export function setPrices(prices: Array<[assetName: string, price: string]>) {
    return {
      set_prices: {
        prices,
      },
    };
  }
}

// CW20
export namespace CW20 {
  export function transfer(recipient: string, amount: string) {
    return {
      transfer: {
        recipient,
        amount,
      },
    };
  }
  export function send(contract: string, amount: string, msgData: any) {
    let msg;
    if (msgData !== undefined) {
      msg = Buffer.from(JSON.stringify(msgData)).toString("base64");
    }
    return {
      send: {
        contract,
        amount,
        msg,
      },
    };
  }
}

export namespace Basket {
  export function setBasketToken(basket_token: string) {
    return {
      __set_basket_token: {
        basket_token,
      },
    };
  }

  export function mint(asset_amounts: string[]) {
    return {
      mint: {
        asset_amounts,
      },
    };
  }

  export function stageAsset() {
    return {
      stage_asset: {},
    };
  }

  export function burn(asset_weights?: number[]) {
    return {
      burn: {
        asset_weights,
      },
    };
  }
}
