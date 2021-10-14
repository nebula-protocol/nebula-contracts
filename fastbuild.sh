#!/bin/bash

docker run --rm -v "$(pwd)":/code \
  -v "$(pwd)/optimize_workspace.py":/usr/local/bin/optimize_workspace.py \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.11.5
