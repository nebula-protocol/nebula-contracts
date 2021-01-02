# Basket Protocol

This monorepository organizes the various components of Basket protocol on Terra.

## Organization

| Name                                    | Type         | Description                                            |
| --------------------------------------- | ------------ | ------------------------------------------------------ |
| [`basket-factory`](#)                   | Contract     | Protocol-level controller contract across many Baskets |
| [`basket-contract`](basket-contract/)   | Contract     | Contract containing individual Basket logic            |
| [`basket-token`](basket-token/)         | Contract     | CW20 Token used for representing ownership of a Basket |
| [`basket-math`](basket-math/)           | Rust Library | Math utility library for Basket protocol               |
| [`terra-wasm-utils`](terra-wasm-utils/) | Rust Library | Generic Terra WASM smart contract utilities            |
| [`basket-webapp`](basket-web-app/)      | Web App      | Rudimentary frontend for Basket protocol               |
