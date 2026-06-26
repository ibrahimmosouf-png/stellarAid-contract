#!/bin/bash
set -e

NETWORK=${1:-testnet}

if [ "$NETWORK" = "testnet" ]; then
  RPC_URL="https://soroban-testnet.stellar.org"
  PASSPHRASE="Test SDF Network ; September 2015"
elif [ "$NETWORK" = "mainnet" ]; then
  RPC_URL="https://soroban.stellar.org"
  PASSPHRASE="Public Global Stellar Network ; September 2015"
else
  echo "Unknown network: $NETWORK. Use testnet or mainnet."
  exit 1
fi

echo "Configuring Soroban network: $NETWORK"
soroban network add "$NETWORK" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$PASSPHRASE" 2>/dev/null || true

echo "Building contracts..."
cargo build --target wasm32-unknown-unknown --release

CONTRACTS=("donation" "withdrawal" "campaign")
for contract in "${CONTRACTS[@]}"; do
  WASM="target/wasm32-unknown-unknown/release/${contract}.wasm"
  echo "Deploying $contract..."
  CONTRACT_ID=$(soroban contract deploy \
    --wasm "$WASM" \
    --network "$NETWORK" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$PASSPHRASE")
  echo "$contract contract ID: $CONTRACT_ID"
done

echo "Deployment to $NETWORK complete."
