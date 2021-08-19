import * as request from 'request-promise';

export class Snapshot {
  URL: string;

  constructor(URL: string) {
    this.URL = URL;
  }

  async takeSnapshot(block: number): Promise<{ [delegator: string]: bigint }> {
    const delegationSnapshot: { [delegator: string]: bigint } = {};
    const validators = JSON.parse(
      await request.get(`${this.URL}/staking/validators?height=${block}`, {
        timeout: 10000000
      })
    )['result'];

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
          `${this.URL}/staking/validators/${operator_addr}/delegations?height=${block}`
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
