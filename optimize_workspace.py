#!/usr/bin/env python3

# Build script for cargo workspaces

CARGO_PATH="cargo"
PACKAGE_PREFIX="contracts/"

import glob
import os
import shutil
import stat
import subprocess
import toml
import threading


def log(*args):
    print(*args, flush=True)


with open("Cargo.toml") as file:
    document = toml.load(file)
    members = document['workspace']['members']

log("Found workspace member entries:", members)

all_packages = []
for member in members:
    all_packages.extend(glob.glob(member))
all_packages.sort()
log("Package directories:", all_packages)

contract_packages = [p for p in all_packages if p.startswith(PACKAGE_PREFIX)]
log("Contracts to be built:", contract_packages)

artifacts_dir = os.path.realpath("artifacts")
os.makedirs(artifacts_dir, exist_ok=True)


def run_on_contract(contract):
    log("Building {} ...".format(contract))
    # make a tmp dir for the output (*.wasm and other) to not touch the host filesystem
    tmp_dir = "/tmp/" + contract
    os.makedirs(tmp_dir, exist_ok=True)

    # Rust nightly and unstable-options is needed to use --out-dir
    cmd = [CARGO_PATH, "-Z=unstable-options", "build", "--release", "--target=wasm32-unknown-unknown", "--locked", "--out-dir={}".format(tmp_dir)]
    os.environ["RUSTFLAGS"] = "-C link-arg=-s"
    subprocess.check_call(cmd, cwd=contract)

    for build_result in glob.glob("{}/*.wasm".format(tmp_dir)):
        log("Optimizing build {} ...".format(build_result))
        name = os.path.basename(build_result)
        cmd = ["wasm-opt", "-Os", "-o", "artifacts/{}".format(name), build_result]
        subprocess.check_call(cmd)


threads = []
for contract in contract_packages:
    threads.append(
        threading.Thread(target=run_on_contract, args=(contract,))
    )
    threads[-1].start()

for thread in threads:
    thread.join()
