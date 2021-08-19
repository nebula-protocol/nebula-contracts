import Airdrop from './Airdrop';

const v1 = [
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '1000000'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000000'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '3000000'
  },
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '1000001'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000002'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '3000003'
  },
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '1000010'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000020'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '3000030'
  }
];

const v2 = [
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '2000000'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000000'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '2000000'
  },
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '2000001'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000001'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '2000001'
  },
  {
    address: 'terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8',
    amount: '2000002'
  },
  {
    address: 'terra1ucp369yry6n70qq3zaxyt85cnug75r7ln8l6se',
    amount: '2000002'
  },
  {
    address: 'terra1t849fxw7e8ney35mxemh4h3ayea4zf77dslwna',
    amount: '2000002'
  }
];

describe('Airdrop', () => {
  it('verify v1', async () => {
    const airdrop = new Airdrop(v1);
    const proof = airdrop.getMerkleProof(v1[3]);

    console.log('Merkle Root', airdrop.getMerkleRoot());
    console.log('Merkle Proof', proof);
    console.log('Target Acc', v1[3]);
    console.log('Verified', airdrop.verify(proof, v1[3]));
  });

  it('verify v2', async () => {
    const airdrop = new Airdrop(v2);
    const proof = airdrop.getMerkleProof(v2[3]);

    console.log('Merkle Root', airdrop.getMerkleRoot());
    console.log('Merkle Proof', proof);
    console.log('Target Acc', v2[3]);
    console.log('Verified', airdrop.verify(proof, v2[3]));
  });
});
