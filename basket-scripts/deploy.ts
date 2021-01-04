import {
  MsgStoreCode,
  MsgExecuteContract,
  MsgInstantiateContract,
  LocalTerra,
  isTxError,
} from "@terra-money/terra.js";

import * as fs from "fs";

const terra = new LocalTerra();
const test1 = terra.wallets.test1;

async function storeContract(contractName: string): Promise<string> {
  console.log(`[storeContract] - ${contractName}`);
  const storeCode = new MsgStoreCode(
    test1.key.accAddress,
    fs.readFileSync(`../artifacts/${contractName}.wasm`).toString("base64")
  );
  const storeCodeTx = await test1.createAndSignTx({
    msgs: [storeCode],
  });
  const storeCodeTxResult = await terra.tx.broadcast(storeCodeTx);
  console.log(storeCodeTxResult);

  if (isTxError(storeCodeTxResult)) {
    throw new Error(
      `store code failed. code: ${storeCodeTxResult.code}, codespace: ${storeCodeTxResult.codespace}, raw_log: ${storeCodeTxResult.raw_log}`
    );
  }

  const {
    store_code: { code_id },
  } = storeCodeTxResult.logs[0].eventsByType;

  return code_id[0];
}

async function instantiateContract(
  codeId: string,
  initMsg: any
): Promise<string> {
  const instantiate = new MsgInstantiateContract(
    test1.key.accAddress,
    +codeId,
    initMsg
  );

  const instantiateTx = await test1.createAndSignTx({
    msgs: [instantiate],
  });
  const instantiateTxResult = await terra.tx.broadcast(instantiateTx);

  console.log(instantiateTxResult);

  if (isTxError(instantiateTxResult)) {
    throw new Error(
      `instantiate failed. code: ${instantiateTxResult.code}, codespace: ${instantiateTxResult.codespace}, raw_log: ${instantiateTxResult.raw_log}`
    );
  }

  const {
    instantiate_contract: { contract_address },
  } = instantiateTxResult.logs[0].eventsByType;

  return contract_address[0];
}

async function main() {
  const tokenCodeId = await storeContract("terraswap_token");
  const oracleCodeId = await storeContract("basket_dummy_oracle");
  const basketCodeId = await storeContract("basket_contract");



  const mAAPLAddress = await instantiateContract(tokenCodeId, {
    symbol: "mAAPL",
    decimals: 6,
    initial_balances: [
      [test1.key.accAddress, 1000000000]
    ],
    mint: {
      minter: test1,
    }
  });
  const mGOOGAddress = await instantiateContract(tokenCodeId, {});
  const mMSFTAddress = await instantiateContract(tokenCodeId, {});
  const mNFLXAddress = await instantiateContract(tokenCodeId, {});
}

name: name.clone(),
                symbol: symbol.to_string(),
                decimals: 6u8,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: deps.api.human_address(&config.mint_contract)?,
                    cap: None,
                }),
                init_hook: Some(InitHook {
                    contract_addr: env.contract.address,
                    msg: to_binary(&HandleMsg::TokenCreationHook { oracle_feeder })?,
                }),