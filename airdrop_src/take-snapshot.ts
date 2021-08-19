// TODO: fix me (add validation)
/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { Coins, LCDClient, MnemonicKey, StdFee, Wallet } from '@terra-money/terra.js'
import { AirdropMerkleItem, Account } from './types'
import { Airdrop } from './Airdrop'
// import { Snapshot } from './Snapshot'

const request = require('request-promise');

// import * as request from 'request-promise';
// const request = require('request');

import * as fs from 'fs'

import { CurrentSnapshot } from './CurrentSnapshot'

const lcd_endpoint = "https://lcd.terra.dev"
const snapshot_stage = 0
const chain_id = "columbus-4"
const luna_staker_airdrop_amount = "100"
console.log("Starting")

void (async function() {
  console.log("Entered function")
  const lcd = new LCDClient({ URL: lcd_endpoint!, chainID: chain_id! })

  // create snapshotService
  const snapshotService = new CurrentSnapshot(lcd_endpoint!)

  const latestBlock = await lcd.tendermint.blockInfo()
  if (!latestBlock) return

  console.log("Latest block", latestBlock)

  // take snapshot, dump to a json file
  const airdropJSON = await takeSnapshot(
    snapshotService,
    chain_id,
    +snapshot_stage,
    0,
    luna_staker_airdrop_amount,
  )

  fs.createWriteStream(`airdrop_${snapshot_stage}.json`).end(JSON.stringify(airdropJSON))

})().catch(console.log)

async function takeSnapshot(
    snapshotService: CurrentSnapshot,
    chainId: string,
    stage: number,
    height: number,
    airdropAmount: string,
): Promise<AirdropMerkleItem[]> {

    // take snapshot
    const snapshotHeight = height
    
    if(snapshotHeight % 100 !== 0) {
      throw new Error(`cannot take snapshot of block ${snapshotHeight}`)
    }

    console.log("About to take snapshot")

    let validators: Array<{
      operator_address: string;
      tokens: string;
      delegator_shares: string;
    }> = JSON.parse(
      await request.get(`${lcd_endpoint}/staking/validators`, {
        timeout: 10000000
      })
    )['result'];

    // Filter out top 5 validators
    validators.sort((a, b) => (parseInt(a.delegator_shares) > parseInt(b.delegator_shares)) ? -1 : 1)
    validators = validators.slice(5, validators.length)

    // const delegators = await snapshotService.takeSnapshot() // change return here to account for new snapshot math
    // console.log("Found delegators (?)")

    const validatorToWeight: { [operator: string]: number } = {};
    let totalValidatorWeight = 0
    for (let i = 0; i < validators.length; i++) {
      let newWeight = parseFloat(validators[i].delegator_shares) ** 0.75
      totalValidatorWeight += newWeight
      validatorToWeight[validators[i].operator_address] = newWeight
    }
    const validatorAddresses = Object.keys(validatorToWeight)

    validatorAddresses.map((validator) => {
      validatorToWeight[validator] /= totalValidatorWeight
    })

    const accounts: Account[] = []
    let totalStaked = BigInt(0)
    for (const [operator_addr, weight] of Object.entries(validatorToWeight)) {
      console.log("Looking at operator", operator_addr)
      const delegators: Array<{
        delegator_address: string;
        validator_address: string;
        shares: string;
        balance: {
          denom: string;
          amount: string;
        };
      }> = JSON.parse(
        await request.get(
          `${lcd_endpoint}/staking/validators/${operator_addr}/delegations`
        )
      )['result'];

      const total = delegators.reduce((s, x) => s + parseFloat(x.shares), 0);
      

      delegators.forEach((delegator) => {
        const staked = delegator.shares
        const rate = (parseFloat(delegator.shares) / total).toString()
        const amount = parseInt(airdropAmount) * parseFloat(rate)

        totalStaked += BigInt(
          delegator.balance.amount
        );

        if (amount > 0) {
            accounts.push({ address: delegator.delegator_address, amount: amount.toString(), staked: staked.toString(), rate })
        };
      })
    }

    // get all
    const airdrop = new Airdrop(accounts)
    const merkleRoot = airdrop.getMerkleRoot()

    // return airdrop data
    const airdropSnapshot = accounts.map(account => {
      const { address, staked, rate, amount } = account
      const proof = airdrop.getMerkleProof({ address, amount })
      const merkleItem: AirdropMerkleItem = {
        stage,
        chainId,
        address,
        staked,
        rate,
        amount,
        total: totalStaked.toString(),
        proof: JSON.stringify(proof),
        claimable: false,
        merkleRoot
      }

      return merkleItem
    })

    return airdropSnapshot
  }