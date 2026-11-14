#!/usr/bin/env bash
# Create a real Sputnik DAO (council bob/council2/council3, 2-of-3) via the testnet factory,
# register it on mUSDC and fund it with 10,000 mUSDC. Idempotent.
source "$(dirname "$0")/env.sh"

echo "== Sputnik DAO $DAO"
if acct_exists "$DAO"; then echo "  = exists"; else
  ARGS=$(jq -nc --arg b "$BOB" --arg c2 "$COUNCIL2" --arg c3 "$COUNCIL3" \
    '{config:{name:"FACTOR demo DAO",purpose:"pays FACTOR invoices",metadata:""},policy:[$b,$c2,$c3]}' | base64 -w0)
  NAME="${DAO%%.*}"
  near_tx "$M" "$SPUTNIK_FACTORY" create "$(jq -nc --arg n "$NAME" --arg a "$ARGS" '{name:$n,args:$a}')" '150 Tgas' '6 NEAR'
  acct_exists "$DAO" || { echo "DAO creation failed:"; echo "$LAST_OUT" | tail -15; exit 1; }
  echo "  + created  tx: $LAST_TX  $(tx_link "$LAST_TX")"
fi
echo "  policy: $(near_view "$DAO" get_policy | jq -c '{council:.roles[]|select(.name=="council").kind.Group, bond:.proposal_bond}')"

echo "== register + fund the DAO on mUSDC"
near_tx "$M" "$MUSDC" storage_deposit "{\"account_id\":\"$DAO\",\"registration_only\":true}" '30 Tgas' '0.0125 NEAR'
near_tx "$M" "$MUSDC" mint "{\"account_id\":\"$DAO\",\"amount\":\"10000000000\"}" '30 Tgas' '0 NEAR'
echo "  DAO mUSDC balance: $(near_view "$MUSDC" ft_balance_of "{\"account_id\":\"$DAO\"}")"

save_deployment dao "$DAO"
echo "== dao ready"

bash "$(dirname "$0")/sync-web.sh"
