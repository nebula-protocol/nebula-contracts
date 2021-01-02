# Basket Protocol

This monorepository organizes the various components of Basket protocol on Terra.

| Name                                              | Type         | Description                                            |
| ------------------------------------------------- | ------------ | ------------------------------------------------------ |
| [`basket-factory`](#)                             | Contract     | Protocol-level controller contract across many Baskets |
| [`basket-contract`](contracts/basket-contract/)   | Contract     | Contract containing individual Basket logic            |
| [`basket-token`](contracts/basket-token/)         | Contract     | CW20 Token used for representing ownership of a Basket |
| [`basket-math`](libraries/basket-math/)           | Rust Library | Math utility library for Basket protocol               |
| [`terra-wasm-utils`](libraries/terra-wasm-utils/) | Rust Library | Generic Terra WASM smart contract utilities            |
| [`basket-webapp`](basket-web-app/)                | Web App      | Rudimentary frontend for Basket protocol               |

## Roadmap

- [ ] penalty function
- [ ] Mirror Protocol integration
- [ ] Terraswap integration

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

### Testing

To run all tests:

```bash
$ ./test.sh
```

### Unit test

To run unit tests for individual contracts, navigate to the contract root directory and run `cargo unit-test`.

```bash
$ cd contracts/basket-contract
$ cargo unit-test
```

### Integration test

To run integration tests, run `cargo integration-test`.

```bash
$ cargo integration-test
```

### End-to-end test

A simple end-to-end test is provided to validate the results of the integration test within a real blockchain environment via LocalTerra.

Make sure LocalTerra is installed and running, then:

```bash
$ ./e2e-test.sh
```

## Deployment

### Upload contract codes

```bash

```

### Instantiate contracts

```bash

```

### Configure web app

The web app is designed to work

```bash

```
