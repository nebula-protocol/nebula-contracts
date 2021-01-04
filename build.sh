#!/bin/bash

CONTRACTS=(
  "basket-contract"
  # "basket-token"
)

# optimized=0
# while getopts "o" opt
# do
#     case $opt in
#     (o) optimized=1 ;;
#     (*) printf "Illegal option '-%s'\n" "$opt" && exit 1 ;;
#     esac
# done

opt_build_contract () {
  echo "[optimized] Building contract $1"
  docker run --rm -v "$(pwd)":/code \
    --mount type=volume,source="devcontract_$1_cache",target=/code/contracts/$1/target \
    --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
    cosmwasm/rust-optimizer:0.10.7 ./contracts/$1
}

for contract in ${CONTRACTS[@]}; do
  # if [ $optimized -eq 1 ]
  # then
    opt_build_contract $contract
  # else
  #   build_contract $contract
  # fi
done