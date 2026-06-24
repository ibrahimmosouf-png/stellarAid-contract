#!/bin/bash
set -e

# This script verifies that the deployed WASM of a contract matches the locally compiled WASM.
# This is a security measure to ensure that the deployed contract has not been tampered with.

# Usage:
# ./scripts/verify-deployment.sh <CONTRACT_ID> <PATH_TO_WASM>

CONTRACT_ID=$1
WASM_PATH=$2

if [ -z "$CONTRACT_ID" ] || [ -z "$WASM_PATH" ]; then
  echo "Usage: ./scripts/verify-deployment.sh <CONTRACT_ID> <PATH_TO_WASM>"
  exit 1
fi

# Get the hash of the deployed contract
echo "Fetching deployed WASM hash for $CONTRACT_ID..."
DEPLOYED_WASM_HASH=$(soroban contract fetch --id "$CONTRACT_ID" --network testnet | grep "WASM hash" | awk '{print $3}')

# Get the hash of the local WASM file
echo "Calculating local WASM hash for $WASM_PATH..."
LOCAL_WASM_HASH=$(soroban contract install --wasm "$WASM_PATH" --network testnet | grep "WASM hash" | awk '{print $3}')

# Compare the hashes
if [ "$DEPLOYED_WASM_HASH" == "$LOCAL_WASM_HASH" ]; then
  echo "✅ Verification successful: Deployed WASM matches local WASM."
else
  echo "❌ Verification failed: Deployed WASM does not match local WASM."
  echo "Deployed WASM hash: $DEPLOYED_WASM_HASH"
  echo "Local WASM hash:   $LOCAL_WASM_HASH"
  exit 1
fi