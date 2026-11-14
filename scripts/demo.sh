#!/usr/bin/env bash
# End-to-end FACTOR story on testnet (PRD §15): 8 steps, every step prints its tx hash + explorer link.
# Re-runnable: each run creates a fresh invoice. Exits non-zero if any expected outcome differs.
source "$(dirname "$0")/env.sh"
[ -s "$DEPLOYMENTS" ] || { echo "run make deploy dao first"; exit 1; }

# fixed demo conversion rate used ONLY for the implied-APR display (also labelled in the UI/README)
RATE_NEAR_PER_MUSDC=0.02      # 1 NEAR = 50 mUSDC
FACE_MUSDC=100; FACE=$((FACE_MUSDC * 1000000))
PRICE_NEAR=1.9; DAYS_TO_DUE=25
FAILS=0
check() { # <description> <actual> <expected>
  if [ "$2" = "$3" ]; then echo "      ✓ $1"; else echo "      ✗ $1 (expected '$3', got '$2')"; FAILS=$((FAILS+1)); fi
}
step() { echo "[$1/8] $2  tx: $LAST_TX  🔗 $(tx_link "$LAST_TX")"; }
inv_field() { near_view "$FACTOR" invoice "{\"id\":\"$1\"}" | jq -r ".$2"; }
musdc_bal() { near_view "$MUSDC" ft_balance_of "{\"account_id\":\"$1\"}" | jq -r .; }
near_bal_yocto() { rpc query "{\"request_type\":\"view_account\",\"finality\":\"final\",\"account_id\":\"$1\"}" | jq -r .result.amount; }

# Sputnik: council member proposes a FunctionCall, two members vote -> executes. Sets PROPOSAL_ID, LAST_TX.
dao_call() { # <receiver> <method> <json args> <deposit yocto> <gas yocto> <description>
  local kind bond pid
  kind=$(jq -nc --arg r "$1" --arg m "$2" --arg a "$(printf '%s' "$3" | base64 -w0)" --arg d "$4" --arg g "$5" \
    '{FunctionCall:{receiver_id:$r,actions:[{method_name:$m,args:$a,deposit:$d,gas:$g}]}}')
  bond=$(near_view "$DAO" get_policy | jq -r .proposal_bond)
  near_tx "$BOB" "$DAO" add_proposal "$(jq -nc --arg d "$6" --argjson k "$kind" '{proposal:{description:$d,kind:$k}}')" '60 Tgas' "$bond yoctoNEAR"
  pid=$(( $(near_view "$DAO" get_last_proposal_id) - 1 ))
  PROPOSAL_ID=$pid
  local vote; vote=$(jq -nc --argjson id "$pid" --argjson k "$kind" '{id:$id,action:"VoteApprove",proposal:$k}')
  near_tx "$BOB" "$DAO" act_proposal "$vote" '300 Tgas' '0 NEAR'
  near_tx "$COUNCIL2" "$DAO" act_proposal "$vote" '300 Tgas' '0 NEAR'   # 2nd vote executes (LAST_TX = executing tx)
}

echo "FACTOR demo  |  factor=$FACTOR  musdc=$MUSDC  dao=$DAO"
echo "(testnet only; mUSDC is a mock token; implied APR uses a fixed demo rate of 1 NEAR = 50 mUSDC)"

# ---- setup that is not part of the story (idempotent): carol must be registered to receive mUSDC
near_tx "$M" "$MUSDC" storage_deposit "{\"account_id\":\"$CAROL\",\"registration_only\":true}" '30 Tgas' '0.0125 NEAR'
near_tx "$M" "$MUSDC" storage_deposit "{\"account_id\":\"$ALICE\",\"registration_only\":true}" '30 Tgas' '0.0125 NEAR'
# keep the buyer solvent: each run moves PRICE_NEAR from carol to alice, so top carol up from alice when low
NEED=$(python3 -c "print(int(($PRICE_NEAR + 1) * 10**24))")
CB=$(near_bal_yocto "$CAROL")
if python3 -c "import sys; sys.exit(0 if int('$CB') < $NEED else 1)"; then
  near tokens "$ALICE" send-near "$CAROL" "$((${PRICE_NEAR%.*} + 1)) NEAR" network-config $NETWORK "${SIGN[@]}" >/dev/null 2>&1     && echo "(topped up carol with $((${PRICE_NEAR%.*} + 1)) NEAR from alice)"
fi
BEFORE_ONTIME=$(near_view "$FACTOR" payer_stats "{\"dao\":\"$DAO\"}" | jq -r '.paid_on_time // 0')
ID=$(near_view "$FACTOR" config | jq -r .next_id)
VAULT="inv-$ID.$FACTOR"
DUE_NS=$(( ($(date +%s) + DAYS_TO_DUE*86400) * 1000000000 ))

# ---- 1. Alice creates the invoice
near_tx "$ALICE" "$FACTOR" create_invoice \
  "$(jq -nc --arg d "$DAO" --arg a "$FACE" --arg t "$DUE_NS" '{payer_dao:$d,amount:$a,due_ts:$t,memo:"Q4 design work"}')" '100 Tgas' '0.1 NEAR'
step 1 "Alice creates invoice #$ID for $FACE_MUSDC mUSDC to ${DAO%%.*}…"
KEYS=$(key_count "$VAULT")
echo "      vault $VAULT  access keys: $KEYS $([ "$KEYS" = 0 ] && echo ✓ || echo ✗)"
check "vault has 0 access keys" "$KEYS" 0
check "invoice DRAFT, owner alice" "$(inv_field "$ID" status)/$(inv_field "$ID" owner)" "Draft/$ALICE"

# ---- 2. DAO accepts by real governance: proposal + 2 of 3 votes
dao_call "$FACTOR" accept_invoice "{\"id\":\"$ID\"}" 0 30000000000000 "accept FACTOR invoice #$ID"
step 2 "Council proposal #$PROPOSAL_ID 'accept invoice #$ID' → 2/3 votes → executed"
check "proposal approved" "$(near_view "$DAO" get_proposal "{\"id\":$PROPOSAL_ID}" | jq -r .status)" Approved
check "invoice ACCEPTED" "$(inv_field "$ID" status)" Accepted

# ---- 3. Alice lists at a discount
PRICE_YOCTO=$(awk -v p="$PRICE_NEAR" 'BEGIN{printf "%d", p*1000}')000000000000000000000
APR=$(awk -v f="$FACE_MUSDC" -v r="$RATE_NEAR_PER_MUSDC" -v p="$PRICE_NEAR" -v d="$DAYS_TO_DUE" 'BEGIN{printf "%.0f", ((f*r/p)-1)*365/d*100}')
near_tx "$ALICE" "$FACTOR" list "{\"id\":\"$ID\",\"price\":\"$PRICE_YOCTO\"}" '30 Tgas' '1 yoctoNEAR'
step 3 "Alice lists #$ID at $PRICE_NEAR NEAR (implied APR ${APR}%)"
check "listed at price" "$(inv_field "$ID" is_listed)/$(inv_field "$ID" price)" "true/$PRICE_YOCTO"

# ---- 4. Carol buys; Alice receives the NEAR today
ALICE_BEFORE=$(near_bal_yocto "$ALICE")
near_tx "$CAROL" "$FACTOR" buy "{\"id\":\"$ID\"}" '50 Tgas' "$PRICE_YOCTO yoctoNEAR"
step 4 "Carol buys #$ID → Alice receives $PRICE_NEAR NEAR"
GAIN=$(python3 -c "print(int('$(near_bal_yocto "$ALICE")')-int('$ALICE_BEFORE'))")
check "alice received exactly the price" "$GAIN" "$PRICE_YOCTO"
check "carol owns the NFT" "$(inv_field "$ID" owner)" "$CAROL"

# ---- 5. DAO pays the vault's address (second real proposal)
dao_call "$MUSDC" ft_transfer_call "{\"receiver_id\":\"$VAULT\",\"amount\":\"$FACE\",\"msg\":\"\"}" 1 200000000000000 "pay FACTOR invoice #$ID"
STATUS=$(inv_field "$ID" status); ONTIME=$(inv_field "$ID" paid_on_time)
step 5 "DAO pays $FACE_MUSDC mUSDC to $VAULT → $(echo "$STATUS" | tr a-z A-Z) ($([ "$ONTIME" = true ] && echo on time || echo late))"
check "invoice PAID on time" "$STATUS/$ONTIME" "Paid/true"
check "vault holds the payment" "$(musdc_bal "$VAULT")" "$FACE"

# ---- 6. Alice (old owner) tries to withdraw -> rejected
near_tx "$ALICE" "$FACTOR" withdraw "{\"id\":\"$ID\"}" '150 Tgas' '1 yoctoNEAR'
step 6 "Alice withdraws → ❌ rejected: NotOwner (expected)"
check "rejection reason is ERR_NOT_OWNER" "$(echo "$LAST_OUT" | grep -c ERR_NOT_OWNER | sed 's/^[1-9].*/found/')" found
check "invoice still PAID, funds still in vault" "$(inv_field "$ID" status)/$(musdc_bal "$VAULT")" "Paid/$FACE"

# ---- 7. Carol (current owner) withdraws
CAROL_BEFORE=$(musdc_bal "$CAROL")
near_tx "$CAROL" "$FACTOR" withdraw "{\"id\":\"$ID\"}" '150 Tgas' '1 yoctoNEAR'
GOT=$(( $(musdc_bal "$CAROL") - CAROL_BEFORE ))
step 7 "Carol withdraws → ✅ $((GOT / 1000000)) mUSDC received"
check "carol received $FACE_MUSDC mUSDC" "$GOT" "$FACE"
check "invoice SETTLED, unlocked" "$(inv_field "$ID" status)/$(inv_field "$ID" pending)" "Settled/false"

# ---- 8. Public payer score
S=$(near_view "$FACTOR" payer_stats "{\"dao\":\"$DAO\"}")
ACC=$(echo "$S" | jq -r .accepted); ONT=$(echo "$S" | jq -r .paid_on_time); LATE=$(echo "$S" | jq -r .paid_late)
VOL=$(echo "$S" | jq -r '.volume_paid|tonumber/1000000')
echo "[8/8] ${DAO%%.*} payer score: $(echo "$S" | jq -r .payer_score)/100 — $ONT/$((ONT+LATE)) on time, accepted $ACC, volume $VOL mUSDC"
check "paid_on_time increased by 1" "$ONT" "$((BEFORE_ONTIME + 1))"

echo
if [ "$FAILS" = 0 ]; then echo "DEMO PASSED ✅"; else echo "DEMO FAILED ❌ ($FAILS checks)"; exit 1; fi
