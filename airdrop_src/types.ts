export class AirdropMerkleItem {
    stage: number;
    chainId: string;
    address: string;
    staked: string;
    rate: string;
    amount: string;
    total: string;
    proof: string;
    claimable: boolean;
    merkleRoot: string;
}

export class Account {
    address: string;
    amount: string;
    staked: string;
    rate: string;
}
