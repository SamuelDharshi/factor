#!/usr/bin/env bash
# Phase 1 spike on testnet: global vault code (by hash) + factor spike_create -> keyless inv-<id>.<spike>.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f .env ] || { echo "missing .env (copy .env.example)"; exit 1; }
set -a; source .env; set +a
NET="network-config ${NEAR_ENV:-testnet}"
SIGN="sign-with-plaintext-private-key $MASTER_PRIVATE_KEY send"
SPIKE="spike.$MASTER_ACCOUNT"
PUB=$(python3 scripts/pubkey.py "$MASTER_PRIVATE_KEY")
VAULT_WASM=target/near/factor_vault/factor_vault.wasm
FACTOR_WASM=target/near/factor/factor.wasm
HASH=$(python3 scripts/codehash.py "$VAULT_WASM")

exists() { near account view-account-summary "$1" $NET now >/dev/null 2>&1; }

echo "== global vault code hash: $HASH"
near contract deploy-as-global use-file "$VAULT_WASM" as-global-hash "$MASTER_ACCOUNT" $NET $SIGN

if ! exists "$SPIKE"; then
  near account create-account fund-myself "$SPIKE" '3 NEAR' use-manually-provided-public-key "$PUB" \
    sign-as "$MASTER_ACCOUNT" $NET $SIGN
  near contract deploy "$SPIKE" use-file "$FACTOR_WASM" with-init-call new \
    json-args "{\"owner_id\":\"$MASTER_ACCOUNT\",\"stablecoin\":\"musdc.$MASTER_ACCOUNT\",\"vault_code_hash\":\"$HASH\"}" \
    prepaid-gas '30 Tgas' attached-deposit '0 NEAR' $NET $SIGN
fi

ID=1; while exists "inv-$ID.$SPIKE"; do ID=$((ID+1)); done
echo "== spike_create($ID)"
near contract call-function as-transaction "$SPIKE" spike_create json-args "{\"id\":$ID}" \
  prepaid-gas '100 Tgas' attached-deposit '0.1 NEAR' sign-as "$MASTER_ACCOUNT" $NET $SIGN

echo "== access keys of inv-$ID.$SPIKE (must be empty)"
near account list-keys "inv-$ID.$SPIKE" $NET now
echo "== get_config"
near contract call-function as-read-only "inv-$ID.$SPIKE" get_config json-args '{}' $NET now
near account view-account-summary "inv-$ID.$SPIKE" $NET now
