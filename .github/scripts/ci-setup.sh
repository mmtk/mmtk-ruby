#!/bin/bash

set -xe

source $(dirname "$0")/common.sh

# Install stable rust
rustup toolchain install $RUSTUP_TOOLCHAIN
rustup component add clippy --toolchain $RUSTUP_TOOLCHAIN
rustup component add rustfmt --toolchain $RUSTUP_TOOLCHAIN

# Install dependencies
sudo apt-get update -y
sudo apt-get install dos2unix build-essential ruby-dev
