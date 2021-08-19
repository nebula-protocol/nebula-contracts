// import * as request from 'request-promise';
const request = require('request-promise');
// const request = require('request');

export class CurrentSnapshot {
  URL: string;

  constructor(URL: string) {
    this.URL = URL;
  }

  async takeSnapshot(): Promise<{ [delegator: string]: bigint }> {
    const delegationSnapshot: { [delegator: string]: bigint } = {};
    console.log('Inside taking snapshot')
    const validators = JSON.parse(
      await request.get(`${this.URL}/staking/validators`, {
        timeout: 10000000
      })
    )['result'];

    // FILTER OUT VALIDATORS?

    console.log("Found " + validators.length + " validators")

    for (let i = 0; i < validators.length; i++) {
      const operator_addr = validators[i]['operator_address'];
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
          `${this.URL}/staking/validators/${operator_addr}/delegations`
        )
      )['result'];

      delegators.forEach((delegation) => {
        if (delegationSnapshot[delegation.delegator_address] === undefined) {
          delegationSnapshot[delegation.delegator_address] = BigInt(0);
        }

        delegationSnapshot[delegation.delegator_address] += BigInt(
          delegation.balance.amount
        );
      });
    }
    
    return delegationSnapshot;
  }
}
