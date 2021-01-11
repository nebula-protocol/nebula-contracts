# Basket Protocol

This monorepository organizes the various components of Basket protocol on Terra.

| Name                                            | Type         | Description                                            |
| ----------------------------------------------- | ------------ | ------------------------------------------------------ |
| [`basket-factory`](#)                           | Contract     | Protocol-level controller contract across many Baskets |
| [`basket-contract`](contracts/basket-contract/) | Contract     | Contract containing individual Basket logic            |
| [`basket-token`](contracts/basket-token/)       | Contract     | CW20 Token used for representing ownership of a Basket |
| [`basket-math`](libraries/basket-math/)         | Rust Library | Math utility library for Basket protocol               |
| [`basket-webapp`](basket-web-app/)              | Web App      | Rudimentary frontend for Basket protocol               |

## Development

**Requirements for contracts**

- Rust 1.44+

  Make sure the target `wasm32-unknown-unknown` is installed.

  ```bash
  $ rustup default stable
  $ rustup target add wasm32-unknown-unknown
  ```

- Docker

**Requirements for web app**

- Node v12+
- NPM / Yarn

### Building

Run the script provided while in the root of the directory.

```bash
$ ./build.sh
```

### Unit test

To run unit tests for individual contracts, navigate to the contract root directory and run `cargo unit-test`.

```bash
$ cd contracts/basket-contract
$ cargo unit-test
```

### Deployment

Provided is a small script to get set up with several dummy Mirrored assets and a test oracle. The basket and oracle will be initialized to the following starting state:

| Asset | Price   | Basket Balance | %              | Target |
| ----- | ------- | -------------- | -------------- | ------ |
| mAAPL | 135.18  | 7290.053159    | 0.2036735938%  | 0.20   |
| mGOOG | 1780.03 | 319.710128     | 0.1176184104%  | 0.10   |
| mMSFT | 222.42  | 14219.28123    | 0.6536466948%  | 0.65   |
| mNFLX | 540.82  | 224.212221     | 0.02506130107% | 0.05   |

The script will then run a `MINT` operation, adding the following:

| Asset | Amount |
| ----- | ------ |
| mAAPL | 125    |
| mGOOG | 0      |
| mMSFT | 149    |
| mNFLX | 50     |

Run the build script and then run:

```bash
$ cd basket-scripts
$ yarn i
$ yarn deploy
```

You can change whether to deploy to LocalTerra or Tequila testnet in `deploy.ts`:

```ts
// UNCOMMENT FOR TEQUILA
const lt = new LocalTerra();
const terra = new LCDClient({
  URL: "https://tequila-lcd.terra.dev",
  chainID: "tequila-0004",
});
const deployer: Wallet = terra.wallet(lt.wallets.test1.key);

// UNCOMMENT FOR LOCALTERRA
// const terra = new LocalTerra();
// const deployer: Wallet = terra.wallets.test1;
```

The deployer uses the LocalTerra `test1` key, so if deploying to Tequila, make sure the address `terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v` has enough tokens by requesting them through [Terra Faucet](https://faucet.terra.money).
