#!/usr/bin/env bash
# Deploy mUSDC, the vault as a GLOBAL contract (by hash) and the factor to testnet. Idempotent.
source "$(dirname "$0")/env.sh"
for f in "$VAULT_WASM" "$FACTOR_WASM" "$MUSDC_WASM"; do [ -f "$f" ] || { echo "missing $f: run make build"; exit 1; }; done

echo "== sub-accounts of $M"
create_subaccount "$FACTOR"   '5 NEAR'
create_subaccount "$MUSDC"    '3 NEAR'
create_subaccount "$ALICE"    '4 NEAR'
create_subaccount "$BOB"      '6 NEAR'
create_subaccount "$CAROL"    '13 NEAR'
create_subaccount "$COUNCIL2" '2 NEAR'
create_subaccount "$COUNCIL3" '1 NEAR'

echo "== mUSDC"
if [ -n "$(near_view "$MUSDC" ft_metadata)" ]; then echo "  = already deployed"; else
  near contract deploy "$MUSDC" use-file "$MUSDC_WASM" with-init-call new json-args '{}' \
    prepaid-gas '30 Tgas' attached-deposit '0 NEAR' network-config $NETWORK "${SIGN[@]}" >/dev/null
  echo "  + deployed"
fi

echo "== vault code as GLOBAL contract (by hash)"
LOCAL="$(python3 scripts/codehash.py "$VAULT_WASM")"
ONCHAIN="$(near_view "$FACTOR" config | jq -r '.vault_code_hash // empty' 2>/dev/null || true)"
if global_exists "$LOCAL"; then
  HASH="$LOCAL"; echo "  = local build is already published"
elif [ -n "$ONCHAIN" ] && global_exists "$ONCHAIN" && [ "${NEW_VAULT:-0}" != 1 ]; then
  # A global deploy BURNS ~11 NEAR per new hash and cargo-near builds are not byte-reproducible,
  # so never redeploy silently: keep the published hash the factor already uses.
  HASH="$ONCHAIN"
  echo "  local build ($LOCAL) is not published; keeping the published hash ($ONCHAIN)."
  echo "  set NEW_VAULT=1 to burn ~11 NEAR and publish the new build."
else
  near contract deploy-as-global use-file "$VAULT_WASM" as-global-hash "$M" network-config $NETWORK "${SIGN[@]}" >/dev/null 2>&1 || true
  sleep 3
  global_exists "$LOCAL" || { echo "ERROR: global deploy of $LOCAL failed (is $M funded with ~12 NEAR?). Factor NOT updated."; exit 1; }
  HASH="$LOCAL"; echo "  + published"
fi
echo "  hash $HASH"

echo "== factor"
if [ -n "$(near_view "$FACTOR" config)" ]; then
  if [ "$ONCHAIN" = "$HASH" ]; then echo "  = already deployed, vault hash unchanged"; else
    echo "  = already deployed; pointing NEW invoices at vault hash $HASH"
    near_tx "$M" "$FACTOR" set_vault_code_hash "{\"vault_code_hash\":\"$HASH\"}" '30 Tgas' '0 NEAR'
  fi
else
  near contract deploy "$FACTOR" use-file "$FACTOR_WASM" with-init-call new \
    json-args "{\"owner_id\":\"$M\",\"stablecoin\":\"$MUSDC\",\"vault_code_hash\":\"$HASH\"}" \
    prepaid-gas '30 Tgas' attached-deposit '0 NEAR' network-config $NETWORK "${SIGN[@]}" >/dev/null
  echo "  + deployed"
fi

echo "== agent registry"
[ -f "$REGISTRY_WASM" ] || { echo "missing $REGISTRY_WASM: run make build"; exit 1; }
create_subaccount "$REGISTRY" '3 NEAR'
if [ -n "$(near_view "$REGISTRY" agents_count)" ]; then echo "  = already deployed"; else
  near contract deploy "$REGISTRY" use-file "$REGISTRY_WASM" with-init-call new json-args '{}'     prepaid-gas '30 Tgas' attached-deposit '0 NEAR' network-config $NETWORK "${SIGN[@]}" >/dev/null
  echo "  + deployed"
fi
save_deployment registry "$REGISTRY"

save_deployment network testnet
save_deployment master "$M"
save_deployment factor "$FACTOR"
save_deployment musdc "$MUSDC"
save_deployment vault_code_hash "$HASH"
save_deployment alice "$ALICE"; save_deployment bob "$BOB"; save_deployment carol "$CAROL"
save_deployment council2 "$COUNCIL2"; save_deployment council3 "$COUNCIL3"
echo "== deployed. addresses in deployments/testnet.json"
jq . "$DEPLOYMENTS"
