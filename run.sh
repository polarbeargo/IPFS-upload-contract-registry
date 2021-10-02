#!/bin/bash
brew update
brew upgrade
brew tap ethereum/ethereum
brew install solidity
cargo run --example add_file file.txt