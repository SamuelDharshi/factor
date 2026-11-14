#!/usr/bin/env bash
# Shared helpers: loads .env, derives account names, wraps near-cli-rs (non-interactive).
# source this file; do not execute it.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
[ -f .env ] || { echo "missing .env: cp .env.example .env and fill MASTER_ACCOUNT / MASTER_PRIVATE_KEY"; exit 1; }
set -a; source .env; set +a
: "${MASTER_ACCOUNT:?set MASTER_ACCOUNT in .env}"
: "${MASTER_PRIVATE_KEY:?set MASTER_PRIVATE_KEY in .env}"
command -v jq >/dev/null || { echo "jq is required"; exit 1; }
command -v near >/dev/null || { echo "near-cli-rs is required (https://github.com/near/near-cli-rs)"; exit 1; }

NETWORK=testnet
RPC=https://rpc.testnet.near.org
EXPLORER=https://testnet.nearblocks.io
SPUTNIK_FACTORY=sputnik-v2.testnet   # verified against sputnik-dao-contract README (see docs/DECISIONS.md)
M="$MASTER_ACCOUNT"
FACTOR="factor.$M"; MUSDC="musdc.$M"
ALICE="alice.$M"; BOB="bob.$M"; CAROL="carol.$M"; COUNCIL2="council2.$M"; COUNCIL3="council3.$M"
DAO="${M%.testnet}-dao.$SPUTNIK_FACTORY"
DEPLOYMENTS="$ROOT/deployments/testnet.json"
VAULT_WASM=target/near/factor_vault/factor_vault.wasm
FACTOR_WASM=target/near/factor/factor.wasm
MUSDC_WASM=target/near/musdc/musdc.wasm
REGISTRY_WASM=target/near/factor_registry/factor_registry.wasm
REGISTRY="registry.$M"; ROVER="rover.$M"; SCOUT="scout.$M"
SIGN=(sign-with-plaintext-private-key "$MASTER_PRIVATE_KEY" send)
MASTER_PUBKEY="$(python3 "$ROOT/scripts/pubkey.py" "$MASTER_PRIVATE_KEY")"

strip() { sed 's/\x1b\[[0-9;]*m//g'; }
tx_link() { echo "$EXPLORER/txns/$1"; }

# rpc <method> <params-json>
rpc() { curl -s -m 30 "$RPC" -H 'content-type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$1\",\"params\":$2}"; }

acct_exists() {
  local r; r=$(rpc query "{\"request_type\":\"view_account\",\"finality\":\"final\",\"account_id\":\"$1\"}")
  [ "$(echo "$r" | jq 'has("error") or (.result|has("error"))')" = "false" ]
}
# global_exists <code hash>: is this global contract actually published on-chain?
global_exists() { rpc query "{\"request_type\":\"view_global_contract_code\",\"finality\":\"final\",\"code_hash\":\"$1\"}" | jq -e '.result.code_base64' >/dev/null 2>&1; }
key_count() { rpc query "{\"request_type\":\"view_access_key_list\",\"finality\":\"final\",\"account_id\":\"$1\"}" | jq '.result.keys|length'; }

# near_view <contract> <method> [json]  -> prints the JSON return value
near_view() {
  local args="${3:-}"; [ -n "$args" ] || args='{}'
  near contract call-function as-read-only "$1" "$2" json-args "$args" network-config $NETWORK now 2>/dev/null \
    | strip   # result JSON is on stdout; the log banner goes to stderr
}

# near_tx <signer> <contract> <method> <json> <gas e.g. '100 Tgas'> <deposit e.g. '1 yoctoNEAR'>
# sets LAST_OUT (full cli output) and LAST_TX (tx hash). Never aborts: callers assert on state / output.
near_tx() {
  LAST_OUT=$(near contract call-function as-transaction "$2" "$3" json-args "$4" prepaid-gas "$5" attached-deposit "$6" \
    sign-as "$1" network-config $NETWORK "${SIGN[@]}" 2>&1 | strip) || true
  LAST_TX=$(echo "$LAST_OUT" | grep -oE '(Transaction ID: |txns?/|transactions/)[A-Za-z0-9]{43,44}' | head -1 | grep -oE '[A-Za-z0-9]{43,44}$' || true)
  [ -n "$LAST_TX" ] || LAST_TX="(no-hash-parsed)"
}

create_subaccount() { # <name> <balance>
  if acct_exists "$1"; then echo "  = $1 exists"; return; fi
  near account create-account fund-myself "$1" "$2" use-manually-provided-public-key "$MASTER_PUBKEY" \
    sign-as "$M" network-config $NETWORK "${SIGN[@]}" >/dev/null 2>&1 || { echo "failed to create $1 (is MASTER_ACCOUNT funded?)"; exit 1; }
  echo "  + created $1 ($2)"
}

save_deployment() { # <key> <value>
  mkdir -p "$ROOT/deployments"; [ -s "$DEPLOYMENTS" ] || echo '{}' > "$DEPLOYMENTS"
  jq --arg k "$1" --arg v "$2" '.[$k]=$v' "$DEPLOYMENTS" > "$DEPLOYMENTS.tmp" && mv "$DEPLOYMENTS.tmp" "$DEPLOYMENTS"
}
