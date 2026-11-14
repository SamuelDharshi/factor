#!/usr/bin/env bash
# Register a few DEMO agents/robots on the testnet registry (real registrations from our demo accounts).
source "$(dirname "$0")/env.sh"
create_subaccount "$ROVER" '1 NEAR'
create_subaccount "$SCOUT" '1 NEAR'
reg() { # <signer> <name> <kind> <desc> <caps json> <min_score> <min_apr> <max_price_yocto>
  [ "$(near_view "$REGISTRY" get_agent "{\"account_id\":\"$1\"}")" != "null" ] && { echo "  = $1 already registered"; return; }
  near_tx "$1" "$REGISTRY" register "$(jq -nc --arg n "$2" --arg k "$3" --arg d "$4" --argjson c "$5" --argjson s "$6" --argjson a "$7" --arg p "$8" \
    '{input:{name:$n,kind:$k,description:$d,capabilities:$c,endpoint:"https://example.com/agents/\($n|ascii_downcase|gsub(" ";"-"))",strategy:{min_score:$s,min_apr_pct:$a,max_price:$p}}}')" '50 Tgas' '0.5 NEAR'
  echo "  + $2 ($3) registered  tx: $LAST_TX  $(tx_link "$LAST_TX")"
}
reg "$CAROL" "Carol Capital" Agent "Buys DAO invoices from payers with a proven on-time record, then waits for settlement." '["buy-invoices","risk-scoring","treasury-watch"]' 70 40 10000000000000000000000000
reg "$SCOUT" "Scout" Agent "Scans listings for the highest implied APR and flags overdue payers." '["scan-listings","alerts"]' 50 80 5000000000000000000000000
reg "$ROVER" "Rover-1" Robot "Field unit that attests real-world delivery so a DAO can accept an invoice." '["attest-delivery","iot-sensor"]' 0 0 0
echo "registered: $(near_view "$REGISTRY" agents_count)"
