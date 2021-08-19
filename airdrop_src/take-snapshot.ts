// TODO: fix me (add validation)
/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { Coins, LCDClient, MnemonicKey, StdFee, Wallet } from '@terra-money/terra.js'
import { AirdropMerkleItem, Account } from './types'
import { Airdrop } from './Airdrop'
// import { Snapshot } from './Snapshot'

import * as fs from 'fs'

import * as request from 'request-promise';
import { CurrentSnapshot } from './CurrentSnapshot'

const lcd_endpoint = "https://lcd.terra.dev"
const snapshot_stage = 0
const chain_id = "columbus-4"
const luna_staker_airdrop_amount = "100"

void (async function() {
  const lcd = new LCDClient({ URL: lcd_endpoint!, chainID: chain_id! })

  // create snapshotService
  const snapshotService = new CurrentSnapshot(lcd_endpoint!)

  const latestBlock = await lcd.tendermint.blockInfo()
  if (!latestBlock) return

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
    const delegators = await snapshotService.takeSnapshot()

    // filtering - staked luna >= 1000
    const delegatorAddresses = Object.keys(delegators)
    if (delegatorAddresses.length < 1) {
        throw new Error('take snapshot failed. target delegators is none.')
    }

    // calculate total staked luna amount
    const total = delegatorAddresses.reduce((s, x) => s + delegators[x], BigInt(0));


    // calculate airdrop amount per account
    const accounts: Account[] = []
    try {
        delegatorAddresses.map((delegator) => {
            const staked = BigInt(delegators[delegator].toString())
            const rate = (staked / total).toString()
            const amount = parseInt(airdropAmount) * parseFloat(rate)

            if (amount > 0) {
                accounts.push({ address: delegator, amount: amount.toString(), staked: staked.toString(), rate })
            }
        })
    } catch(error) {
        throw new Error(error)
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
        total: total.toString(),
        proof: JSON.stringify(proof),
        claimable: false,
        merkleRoot
      }

      return merkleItem
    })

    return airdropSnapshot
    }