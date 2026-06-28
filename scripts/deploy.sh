#!/bin/bash
set -e

NETWORK=${1:-testnet}
ADMIN_SECRET=${2:-$STELLAR_PLATFORM_SECRET}

if [ -z "$ADMIN_SECRET" ]; then
  echo "Usage: $0 [network] [admin_secret]"
  echo "Or set STELLAR_PLATFORM_SECRET environment variable."
  exit 1
fi

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

CONFIG_FILE="config/${NETWORK}_contracts.json"

declare -A CONTRACT_IDS

# Deploy in dependency order: campaign -> donation -> withdrawal
CONTRACTS=("campaign" "donation" "withdrawal")
for contract in "${CONTRACTS[@]}"; do
  WASM="target/wasm32-unknown-unknown/release/${contract}.wasm"
  echo "Deploying $contract..."
  CONTRACT_ID=$(soroban contract deploy \
    --wasm "$WASM" \
    --network "$NETWORK" \
    --rpc-url "$RPC_URL" \
    --network-passphrase "$PASSPHRASE" \
    --source "$ADMIN_SECRET")
  CONTRACT_IDS[$contract]=$CONTRACT_ID
  echo "$contract contract ID: $CONTRACT_ID"
done

echo ""
echo "Initializing contracts..."

# 1. Initialize Campaign
echo "Initializing Campaign contract..."
soroban contract invoke \
  --id "${CONTRACT_IDS[campaign]}" \
  --network "$NETWORK" \
  --source "$ADMIN_SECRET" \
  -- \
  initialize \
  --admin "$(soroban keys address --network "$NETWORK" --source "$ADMIN_SECRET")"

# 2. Initialize Donation (depends on campaign contract)
echo "Initializing Donation contract..."
soroban contract invoke \
  --id "${CONTRACT_IDS[donation]}" \
  --network "$NETWORK" \
  --source "$ADMIN_SECRET" \
  -- \
  initialize \
  --admin "$(soroban keys address --network "$NETWORK" --source "$ADMIN_SECRET")" \
  --campaign_contract "${CONTRACT_IDS[campaign]}"

# 3. Initialize Withdrawal (depends on donation contract)
echo "Initializing Withdrawal contract..."
soroban contract invoke \
  --id "${CONTRACT_IDS[withdrawal]}" \
  --network "$NETWORK" \
  --source "$ADMIN_SECRET" \
  -- \
  initialize \
  --admin "$(soroban keys address --network "$NETWORK" --source "$ADMIN_SECRET")" \
  --donation_contract "${CONTRACT_IDS[donation]}"

echo ""
echo "Verifying deployment..."

for contract in "${CONTRACTS[@]}"; do
  echo "Verifying $contract..."
  case $contract in
    campaign)
      soroban contract invoke \
        --id "${CONTRACT_IDS[campaign]}" \
        --network "$NETWORK" \
        --source "$ADMIN_SECRET" \
        -- \
        get_campaign \
        --campaign_id 1 2>/dev/null || echo "  (no campaigns yet - expected)"
      ;;
    donation)
      TOTAL=$(soroban contract invoke \
        --id "${CONTRACT_IDS[donation]}" \
        --network "$NETWORK" \
        --source "$ADMIN_SECRET" \
        -- \
        get_total_raised \
        --campaign_id 1 2>/dev/null || echo "0")
      echo "  Total raised for campaign 1: $TOTAL"
      ;;
    withdrawal)
      COUNT=$(soroban contract invoke \
        --id "${CONTRACT_IDS[withdrawal]}" \
        --network "$NETWORK" \
        --source "$ADMIN_SECRET" \
        -- \
        get_withdrawals_by_campaign \
        --campaign_id 1 2>/dev/null || echo "[]")
      echo "  Withdrawals for campaign 1: $COUNT"
      ;;
  esac
done

echo ""
echo "Saving contract IDs to $CONFIG_FILE..."
cat > "$CONFIG_FILE" << EOF
{
  "network": "$NETWORK",
  "rpc_url": "$RPC_URL",
  "network_passphrase": "$PASSPHRASE",
  "contracts": {
    "campaign": { "id": "${CONTRACT_IDS[campaign]}" },
    "donation": { "id": "${CONTRACT_IDS[donation]}" },
    "withdrawal": { "id": "${CONTRACT_IDS[withdrawal]}" }
  }
}
EOF

echo ""
echo "Deployment to $NETWORK complete!"
echo "Contract IDs saved to $CONFIG_FILE"
