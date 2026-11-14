# FACTOR: Full Product Requirements Document
### "Get paid by DAOs today, not in 6 weeks."

| Field | Value |
|---|---|
| Hackathon | NEAR Hackathon (nearhackathon.com), "Build what's next" |
| Deadline | **14 Nov 2026, 23:59 UTC** (target submit: 22:00 UTC) |
| Tracks | **NFTs That Do Something** (primary) · **Social Money** (secondary) |
| Prize target | Track 1st ($10K) in each selected track; grand prize contender ($55K) |
| Team | 1–2 devs, about 30 hours |
| Network | NEAR testnet (mainnet is listed as Future) |
| Status | Ready to build |

---

## Table of Contents
1. Executive Summary
2. Judging Criteria Map (how each section scores points)
3. Problem
4. Solution & Core Mechanism
5. Why NEAR
6. Users, Personas & User Stories
7. User Flows
8. Functional Requirements
9. Non-Functional Requirements
10. System Architecture
11. Smart Contract Specification
12. Data Models
13. Security Requirements & Threat Model
14. Frontend Specification
15. Scripts, CLI & Developer Experience
16. Testing Plan
17. Demo Script & Video Script
18. README Template (passes the automated screen)
19. Build Plan (30 hours)
20. Risks & Fallbacks
21. Judge Q&A Prep
22. Success Metrics
23. Submission Package (form, X post)
24. Future Roadmap

---

## 1. Executive Summary

DAO contributors on NEAR do the work, then wait weeks for a payout proposal to be created, voted on and executed. **FACTOR turns a DAO-approved invoice into an NFT that is its own bank account.**

- The DAO approves the invoice by onchain vote, which is a public debt acknowledgment.
- The contributor sells the invoice NFT at a discount for instant cash.
- The DAO later pays the invoice's human-readable address (`inv-42.factor.near`).
- **Whoever holds the NFT gets paid.**
- Every DAO builds a public **payer score**, so buyers price real, observable credit risk.

**One sentence for judges:**
> "On NEAR an NFT can own a named bank account. FACTOR uses that to turn DAO-approved invoices into money you can sell."

---

## 2. Judging Criteria Map

The site lists past NEAR criteria as a guide: **technical implementation, design, impact, originality, sustainability**. There is also an automated "does it run as the README says" screen before human judging.

| Criterion | What judges want | How FACTOR delivers | PRD section |
|---|---|---|---|
| **Gate 0: Automated screen** | Repo runs; README claims are true | `make demo` from a fresh clone; every claim backed by a test or a tx hash; no keys needed | §15, §16, §18 |
| **Technical implementation** | Deep, correct use of NEAR, not superficial | Keyless sub-accounts, global contracts, cross-contract calls with callbacks, NEP-171/177/141/145/297, a Sputnik FunctionCall governance flow | §10, §11, §13 |
| **Design** | Clean UX, a clear story, a polished demo | 3-screen UI, implied APR, DAO payer card, status timeline, a 2-minute video with one memorable moment | §14, §17 |
| **Impact** | Real users, real problem | DAO contributors and grant recipients waiting weeks; DAOs wanting net-30 terms; buyers wanting yield from real activity | §3, §6 |
| **Originality** | Not what 20 other teams build | Receivables where **a DAO is the onchain debtor**; NFTs that own named accounts natively | §4, §5 |
| **Sustainability** | Path past the hackathon | Protocol fee on factoring, a payer-score data layer, a Trezu integration path, mainnet USDC + Intents | §24 |

---

## 3. Problem

### 3.1 The pain
1. **Contributors wait.** DAO payouts need a proposal, a quorum, an execution and treasury liquidity. Two to six weeks is normal, and contributors carry the cost.
2. **An approved payout isn't an asset.** Even after a DAO approves payment, the contributor can't sell or borrow against it, because the approval isn't transferable.
3. **DAOs can't offer payment terms cleanly.** A DAO waiting on vesting unlocks or token sales has no standard way to say "approved now, paid in 30 days".
4. **Credit risk is invisible offchain.** Traditional invoice factoring needs paperwork because the payer's ability to pay is opaque.

### 3.2 Why now / why onchain
A DAO's **treasury balance, governance votes and payout history are all public onchain**. That's the missing piece factoring has always needed: a debtor whose creditworthiness anyone can check. NEAR's account model lets each invoice be its own named, keyless account.

### 3.3 Market signal
Invoice factoring is a large, established financing category in traditional finance. Crypto-native organizations (DAOs, protocols, grant programs) pay contributors constantly but have no receivables market.

---

## 4. Solution & Core Mechanism

### 4.1 The loop
```
Contributor creates invoice ──► Vault account + NFT minted (0 keys)
        │
DAO votes to accept (FunctionCall proposal) ──► Onchain debt acknowledgment
        │
Contributor lists at discount ──► Buyer buys ──► Contributor has cash TODAY
        │
DAO pays vault address (mUSDC) ──► Status PAID ──► DAO payer score updated
        │
NFT holder withdraws ──► SETTLED   (old holder's withdraw FAILS)
```

### 4.2 Mechanism properties
| Property | How it's guaranteed |
|---|---|
| The invoice is a real debt | Only the named DAO account can call `accept_invoice`, which in practice happens through a passed governance proposal |
| Money follows the NFT | The vault has **zero access keys**; only the factor contract can trigger `withdraw_to`, and only for the current NFT owner |
| No transfer/withdraw race | A `pending` lock blocks transfer, list and buy during an in-flight withdraw; the callback always clears it |
| No fake payments | The vault accepts only `ft_on_transfer` from the whitelisted stablecoin contract |
| Risk can be priced | Public payer score per DAO plus a live treasury balance, shown with implied APR on every listing |

---

## 5. Why NEAR

| NEAR primitive | How FACTOR uses it | Ethereum equivalent |
|---|---|---|
| **Named sub-accounts** | Each invoice is `inv-42.factor.near`, an address a human can pay by name | ERC-6551 token-bound accounts plus a registry (bolted on) |
| **Keyless contract accounts** | The vault has no keys, so only code moves its funds | A contract wallet (deployment overhead per account) |
| **Global contracts** | One vault code hash shared by every invoice, so creating an invoice is cheap | Proxy/clone patterns |
| **Sputnik DAO** | A real governance vote calls `accept_invoice` | Governor + Timelock setup |
| **Async cross-contract calls + callbacks** | Withdraw lock / release pattern | Synchronous calls (a different risk profile) |
| **Storage staking (NEP-145)** | The vault is pre-registered on the stablecoin at mint | N/A |

---

## 6. Users, Personas & User Stories

### Personas
| Persona | Description | Goal | Pain today |
|---|---|---|---|
| **Alice, contributor** | Freelance dev paid by NEAR DAOs | Cash flow now | Waits 3–6 weeks per payout |
| **Marketing DAO** | Sputnik DAO with a 3–7 member council | Approve work now, pay on schedule, look trustworthy | No way to commit to terms; contributors leave |
| **Carol, buyer** | Holds stablecoins and wants real yield | Buy short-dated receivables with visible risk | Yield sources are opaque or circular |

### User stories (with acceptance criteria)
| ID | As a… | I want… | Acceptance criteria |
|---|---|---|---|
| US-1 | Contributor | to create an invoice to a DAO | Vault account exists with 0 keys; NFT minted to me; status DRAFT |
| US-2 | DAO council | to approve an invoice by vote | Proposal passes, then `accept_invoice` executes; status ACCEPTED; DAO stats `accepted` +1 |
| US-3 | Contributor | to list my invoice at a discount | Listing visible with price, discount %, implied APR |
| US-4 | Buyer | to see the DAO's payer score and treasury before buying | Payer card shows on-time %, volume, treasury mUSDC balance |
| US-5 | Buyer | to buy an invoice | Seller receives NEAR; I own the NFT; listing removed |
| US-6 | DAO | to pay the invoice by address | Transferring ≥ amount to the vault sets status PAID; excess refunded |
| US-7 | NFT holder | to withdraw the payment | Funds arrive; status SETTLED; non-owners rejected |
| US-8 | Anyone | to see overdue invoices | Past due and unpaid shows OVERDUE; a late payment counts as `late` |

---

## 7. User Flows

### 7.1 Happy path (sequence)
```
Alice        factor.testnet         inv-42 vault      DAO (sputnik)      musdc        Carol
  |-- create_invoice ->|                 |                  |               |            |
  |                    |-- create acct + global code ->|    |               |            |
  |                    |-- storage_deposit(vault) ----------------------->  |            |
  |<-- NFT #42 --------|                 |                  |               |            |
  |                    |<-------- accept_invoice(42) [via proposal] --------|            |
  |-- list(42, 9.5N) ->|                 |                  |               |            |
  |                    |<----------------------------------------------- buy(42) +9.5N --|
  |<-- 9.5 NEAR -------|-- nft_transfer -> Carol            |               |            |
  |                    |                 |<-- ft_transfer_call(500) ------->|            |
  |                    |<-- mark_paid ---|                  |               |            |
  |-- withdraw(42) --->| ✗ NotOwner      |                  |               |            |
  |                    |<------------------------------------------------ withdraw(42) -|
  |                    |-- withdraw_to(Carol) ->|-- ft_transfer 500 ------->|-- Carol    |
  |                    |<-- on_withdraw ✓ ------|           |               |            |
```

### 7.2 Edge flows
| Flow | Expected behaviour |
|---|---|
| DAO pays less than due | Full refund; status unchanged (no partial payments in the MVP) |
| Someone calls the vault's `ft_on_transfer` directly | Rejected (caller isn't mUSDC) |
| A non-DAO account calls `accept_invoice` | Rejected |
| Owner transfers the NFT during a withdraw | Rejected (pending) |
| Withdraw transfer fails | Callback clears pending; status stays PAID; can retry |
| Buy below the listed price | Rejected; deposit refunded |
| Buy own listing | Rejected |
| List a DRAFT invoice | Rejected (must be ACCEPTED) |

---

## 8. Functional Requirements

### Must have (MVP: demo and screen)
| ID | Requirement |
|---|---|
| FR-1 | `create_invoice` creates sub-account `inv-{id}.{factor}` with vault code, **no access keys**, pre-registers storage on mUSDC, mints a NEP-171 NFT to the caller |
| FR-2 | `accept_invoice(id)` callable only by `payer_dao`; demonstrated through a real Sputnik FunctionCall proposal on testnet |
| FR-3 | `list`, `delist`, `buy` (NEAR) with owner and status checks |
| FR-4 | Vault accepts mUSDC via `ft_on_transfer` only from the whitelisted token; amount ≥ due, refunds excess; calls `mark_paid` |
| FR-5 | `withdraw` by owner with `pending` lock + callback |
| FR-6 | Per-DAO payer stats: accepted, paid_on_time, paid_late, volume_paid, first_seen |
| FR-7 | NEP-177 metadata reflects status, amount, due, payer, payer score (computed in `nft_token`) |
| FR-8 | Status machine DRAFT → ACCEPTED → (LISTED) → PAID → SETTLED, with OVERDUE computed |
| FR-9 | `make build`, `make test`, `make demo` |

### Should have
| ID | Requirement |
|---|---|
| FR-10 | Web UI (3 screens, §14) |
| FR-11 | `proposal.json` generator: CLI prints the exact Sputnik proposal args to accept or pay an invoice (Trezu-friendly) |
| FR-12 | NEP-297 events for every state change (`invoice_created`, `invoice_accepted`, `invoice_listed`, `invoice_sold`, `invoice_paid`, `invoice_settled`) |

### Won't have (README "Future")
Mainnet, real USDC, NEAR Intents cross-chain payment, fractional invoices, a lending pool, DAO collateral, partial payments.

---

## 9. Non-Functional Requirements
| Area | Requirement |
|---|---|
| Reproducibility | Fresh clone + `.env` with a funded testnet account, then `make demo` completes in < 3 min |
| No external keys | No mainnet funds, no API keys, no LLM needed |
| Gas | Every call ≤ 300 TGas; `create_invoice` ≤ 100 TGas |
| Storage | Invoice creation cost ≤ 0.5 NEAR with global contracts (≤ 2 NEAR fallback); excess deposit refunded |
| Toolchain | Pinned: Rust stable, `cargo-near`, `near-sdk` 5.x, `near-cli-rs`, `near-workspaces` (Rust) |
| Observability | Every demo step prints a tx hash plus a testnet explorer link |
| Code quality | `cargo clippy` clean; tests in CI (GitHub Actions) |

---

## 10. System Architecture

```
┌────────────────────────────── NEAR testnet ───────────────────────────────┐
│                                                                           │
│  factor.<you>.testnet  (NFT + market + stats)                             │
│     ├── inv-1.factor.<you>.testnet   [vault, 0 keys, global code]         │
│     ├── inv-2.factor.<you>.testnet   [vault, 0 keys, global code]         │
│     └── …                                                                 │
│                                                                           │
│  musdc.<you>.testnet   (NEP-141 mock stablecoin + faucet)                 │
│  marketing.sputnik-dao.testnet  (real Sputnik v2 DAO, 3 council, 2-of-3)  │
└───────────────────────────────────────────────────────────────────────────┘
          ▲                         ▲                         ▲
   scripts/demo.sh            web/ (Next.js)            tests/ (near-workspaces)
   near-cli-rs                wallet-selector           sandbox
```

**Components**
- `contracts/factor`: core logic (Rust).
- `contracts/vault`: tiny per-invoice contract (Rust), deployed once as a global contract.
- `contracts/musdc`: NEP-141 + `mint` faucet (testnet only).
- `scripts/`: setup and demo.
- `web/`: optional UI.
- `tests/`: integration.

---

## 11. Smart Contract Specification

### 11.1 `factor` contract

**Init**
```rust
#[init] fn new(owner_id: AccountId, stablecoin: AccountId, vault_code_hash: Base58CryptoHash /* or Vec<u8> fallback */) -> Self
```

**Change methods**
| Method | Caller check | Deposit | Logic |
|---|---|---|---|
| `create_invoice(payer_dao: AccountId, amount: U128, due_ts: U64, memo: String) -> TokenId` | any | ≥ `CREATE_COST` | id++; `Promise::new(inv-id.factor).create_account().transfer(VAULT_FUND).use_global_contract(hash).function_call("new", {factor, stablecoin, amount_due})`; then `musdc.storage_deposit(inv-id)`; mint NFT; status DRAFT; emit event; refund excess |
| `accept_invoice(id)` | `predecessor == payer_dao` | 0 | DRAFT → ACCEPTED; `stats[dao].accepted += 1` |
| `list(id, price: U128)` | owner | 1 yocto | status ∈ {ACCEPTED} & !pending; sets listing |
| `delist(id)` | owner | 1 yocto | removes listing |
| `buy(id)` | ≠ owner | ≥ price | transfer NFT; send price to seller; refund excess; clear listing |
| `withdraw(id)` | owner | 1 yocto | status == PAID & !pending; pending = true; `vault.withdraw_to(owner)` then `.then(self.on_withdraw(id, owner))` |
| `on_withdraw(id, owner)` | `#[private]` | — | pending = false; if success: SETTLED + event |
| `mark_paid(id, amount: U128)` | `predecessor == inv-id.factor` | 0 | status ∈ {ACCEPTED} → PAID; `paid_ts = now`; if `now ≤ due_ts` then `paid_on_time += 1` else `paid_late += 1`; `volume += amount`; clear listing |
| `nft_transfer` / `nft_transfer_call` | NEP-171 | 1 yocto | **require !pending**; clear listing |

**View methods**
`invoice(id) -> InvoiceView`, `payer_stats(dao) -> PayerStatsView`, `listings(from, limit)`, `invoices_by_owner(owner)`, `nft_token`, `nft_tokens`, `nft_metadata`.

### 11.2 `vault` contract (global code)
```rust
#[init] fn new(factor: AccountId, stablecoin: AccountId, invoice_id: TokenId, amount_due: U128)
fn ft_on_transfer(sender_id, amount: U128, msg: String) -> PromiseOrValue<U128>
   // require predecessor == stablecoin
   // require !paid
   // if amount < due -> return amount (full refund)
   // paid = true; call factor.mark_paid(id, due); return amount - due (refund excess)
fn withdraw_to(to: AccountId) -> Promise
   // require predecessor == factor
   // ft_transfer(to, balance) with 1 yocto
```

### 11.3 `musdc` contract
Standard NEP-141 + NEP-145. `mint(to, amount)` open faucet (README: testnet only). 6 decimals.

### 11.4 Events (NEP-297, standard `factor`, version `1.0.0`)
`invoice_created{id, creator, payer_dao, amount, due_ts}` · `invoice_accepted{id, dao}` · `invoice_listed{id, price}` · `invoice_sold{id, from, to, price}` · `invoice_paid{id, amount, on_time}` · `invoice_settled{id, to, amount}`

---

## 12. Data Models

```rust
pub enum Status { Draft, Accepted, Paid, Settled }   // Listed & Overdue are derived

pub struct Invoice {
    id: TokenId,
    creator: AccountId,
    payer_dao: AccountId,
    vault: AccountId,          // inv-{id}.factor...
    amount: u128,              // in stablecoin base units
    due_ts: u64,               // ns
    created_ts: u64,
    accepted_ts: Option<u64>,
    paid_ts: Option<u64>,
    status: Status,
    pending: bool,
    memo: String,
}

pub struct Listing { id: TokenId, seller: AccountId, price: u128 /* yoctoNEAR */ }

pub struct PayerStats {
    accepted: u32, paid_on_time: u32, paid_late: u32,
    volume_paid: u128, first_seen_ts: u64,
}
```

**Derived values (computed in views and the UI)**
- `is_overdue = status == Accepted && now > due_ts`
- `on_time_rate = paid_on_time / (paid_on_time + paid_late)`
- `implied_apr = (face_value / price − 1) × 365 / days_to_due`. The MVP demo prices face and price in the same unit; the UI uses a fixed demo NEAR/mUSDC rate, which the README must state.
- `payer_score` (0–100) = 60 × on_time_rate + 25 × min(1, log10(volume)/6) + 15 × min(1, account_age_days/180). It's volume- and age-weighted, so tiny fake histories score low.

**Storage layout:** `LookupMap<TokenId, Invoice>`, `IterableMap<TokenId, Listing>`, `LookupMap<AccountId, PayerStats>`, NEP-171 `NonFungibleToken` core.

---

## 13. Security Requirements & Threat Model

| # | Threat | Mitigation | Test |
|---|---|---|---|
| T1 | Fake payment: attacker calls vault `ft_on_transfer` directly | `require!(predecessor == stablecoin)` | `t_fake_payment_rejected` |
| T2 | Fake `mark_paid` from a random account | `require!(predecessor == invoice.vault)` | `t_mark_paid_only_vault` |
| T3 | Non-DAO accepts the invoice | `require!(predecessor == payer_dao)` | `t_accept_only_dao` |
| T4 | Old owner withdraws after selling | Owner check at call time | `t_old_owner_withdraw_fails` |
| T5 | Transfer during an in-flight withdraw (double claim) | `pending` blocks transfer, list, buy and withdraw | `t_transfer_blocked_while_pending` |
| T6 | Withdraw fails and funds get stuck in a locked state | Callback clears `pending` on failure | `t_withdraw_failure_unlocks` |
| T7 | Vault keys added later | Vault created with 0 keys; no add-key method exists | demo prints `list-keys` = empty |
| T8 | Underpayment marks PAID | amount < due → full refund | `t_underpay_refunded` |
| T9 | Double payment | `paid` flag → second payment fully refunded | `t_double_pay_refunded` |
| T10 | Payer-score sybil (fake DAO with tiny paid invoices) | Score weighted by volume and account age; UI shows the treasury balance | documented + unit test on the score function |
| T11 | Storage griefing via spam invoices | `create_invoice` requires a deposit covering all storage | `t_create_requires_deposit` |
| T12 | Stale listing after transfer | Listing cleared on any transfer or payment | `t_listing_cleared_on_transfer` |

**Out of scope (disclosed):** a DAO that simply never pays. Mitigation is the public OVERDUE status plus score damage; DAO collateral is Future.

---

## 14. Frontend Specification (Next.js + `@near-wallet-selector`)

### Screen 1: Create invoice (`/new`)
- Fields: payer DAO (autocomplete `*.sputnik-dao.testnet`), amount (mUSDC), due date, memo.
- On submit: wallet tx `create_invoice` → redirect to `/invoice/[id]`.
- Below the form: "Ask your DAO to approve" button that copies the proposal JSON (FR-11).

### Screen 2: Marketplace (`/`)
Card grid. Each card shows:
- Invoice #, payer DAO name and **payer score badge** (green ≥ 75, amber 50–74, red < 50)
- Face value, price, **discount %**, **implied APR**, days to due
- DAO treasury balance (live view call to `musdc.ft_balance_of(dao)`)
- "Buy" button

Filters: min APR, min payer score, due within N days.

### Screen 3: Invoice detail (`/invoice/[id]`)
- **Status timeline:** Created → Accepted (link to the DAO proposal) → Sold → Paid → Settled, with tx links.
- Vault panel: address, "0 access keys ✓", balance.
- Payer card: on-time %, volume, account age, treasury.
- Owner actions: List / Delist / Withdraw (disabled with a tooltip when not allowed).

### Design principles
- One accent color. Status chips use consistent colors.
- Every action shows a tx link toast.
- No dead buttons: anything shown works, so the README stays truthful.

---

## 15. Scripts, CLI & Developer Experience

### Makefile
```make
build:   ## build all contracts (cargo near build)
test:    ## unit + integration tests (sandbox, no network)
deploy:  ## deploy musdc, global vault code, factor to testnet
dao:     ## create Sputnik DAO (3 council, 2-of-3), fund with mUSDC
demo:    ## run full end-to-end story on testnet, print tx links
web:     ## run UI locally
```

### `.env.example`
```
NEAR_ENV=testnet
MASTER_ACCOUNT=yourname.testnet
MASTER_PRIVATE_KEY=ed25519:...
# demo accounts are created as sub-accounts automatically
```

### `scripts/demo.sh` output (target)
```
[1/8] Alice creates invoice #1 for 500 mUSDC to marketing.sputnik-dao…  tx: <hash>  🔗
      vault inv-1.factor.x.testnet  access keys: 0 ✓
[2/8] Council proposal #7 'accept invoice #1' → 2/3 votes → executed      tx: <hash>  🔗
[3/8] Alice lists #1 at 9.5 NEAR (implied APR 78%)                        tx: <hash>  🔗
[4/8] Carol buys #1 → Alice receives 9.5 NEAR                             tx: <hash>  🔗
[5/8] DAO pays 500 mUSDC to inv-1.factor.x.testnet → PAID (on time)       tx: <hash>  🔗
[6/8] Alice withdraws → ❌ rejected: NotOwner (expected)                  tx: <hash>  🔗
[7/8] Carol withdraws → ✅ 500 mUSDC received                             tx: <hash>  🔗
[8/8] marketing.sputnik-dao payer score: 1/1 on time, volume 500
DEMO PASSED ✅
```

---

## 16. Testing Plan

### Unit tests (factor)
- State machine transitions (valid and invalid).
- Payer score function (edge cases: zero history, tiny volume, new account).
- Listing math and deposit refunds.

### Integration tests (`near-workspaces`, sandbox)
| Test | Asserts |
|---|---|
| `t_happy_path` | full flow ends SETTLED; buyer balance +500 |
| `t_vault_has_no_keys` | `view_access_keys(vault)` empty |
| `t_accept_only_dao` | random caller fails |
| `t_accept_via_sputnik_proposal` | real sputnikdao2 wasm in sandbox; proposal → vote → executes accept |
| `t_fake_payment_rejected` | direct `ft_on_transfer` call fails |
| `t_mark_paid_only_vault` | random caller fails |
| `t_underpay_refunded` / `t_double_pay_refunded` | balances correct |
| `t_old_owner_withdraw_fails` | NotOwner |
| `t_transfer_blocked_while_pending` | fails while pending |
| `t_withdraw_failure_unlocks` | pending false after failed transfer |
| `t_listing_cleared_on_transfer` | listing gone |
| `t_late_payment_counts_late` | sandbox fast-forward past due → `paid_late == 1` |

**CI:** GitHub Actions runs `make build test` on push, and the README shows the badge.

---

## 17. Demo & Video Script (2:00–2:30)

| Time | Visual | Voiceover |
|---|---|---|
| 0:00–0:12 | Title card | "DAO contributors on NEAR do the work, then wait weeks to get paid. FACTOR fixes that." |
| 0:12–0:30 | Create invoice → vault account, **0 keys** | "Alice invoices Marketing DAO for 500 USDC. FACTOR mints an NFT that owns its own NEAR account, inv-1.factor. It has no keys, so only code can move its money." |
| 0:30–0:50 | Sputnik proposal + votes | "The DAO approves the invoice by onchain vote. That's a public debt acknowledgment from a payer whose treasury anyone can see." |
| 0:50–1:10 | Marketplace card: APR, payer score | "Alice lists it at a discount. Carol sees a 78% implied APR, the DAO's treasury and its repayment record, and buys. Alice gets paid today." |
| 1:10–1:30 | DAO pays address → PAID | "Later the DAO pays the invoice's address, just a name." |
| 1:30–1:50 | **Alice withdraw ❌ / Carol withdraw ✅** | "Alice can't withdraw anymore. The money follows the NFT. Carol withdraws 500." |
| 1:50–2:10 | Payer score updates; architecture slide | "Every DAO builds a public payer score. On NEAR, named sub-accounts and keyless contracts make this native. No ERC-6551 needed." |
| 2:10–2:25 | Terminal: `make demo` → DEMO PASSED | "One command reproduces everything on testnet. FACTOR: get paid by DAOs today." |

---

## 18. README Template (written for the automated screen)

```markdown
# FACTOR: DAO-approved invoices as NFTs that are their own bank account
Built at NEAR HACKATHON. Tracks: NFTs That Do Something, Social Money.

## What it does (every line below is verified by `make test` or a testnet tx)
- Mints an invoice NFT whose vault is a keyless NEAR sub-account (tx: …)
- Only the named DAO can accept, demonstrated via a Sputnik proposal (tx: …)
- List / buy invoices at a discount (tx: …)
- DAO pays the vault address in mUSDC → invoice PAID (tx: …)
- Only the current NFT owner can withdraw (old owner rejected: tx: …)
- Public payer score per DAO

## Quick start
cp .env.example .env    # add a funded testnet account
make build && make test # sandbox tests, no network needed
make deploy dao demo    # full story on testnet, prints tx links

## Deployed (testnet)
factor: … | musdc: … | vault code hash: … | DAO: …

## Architecture   (diagram)
## Security       (threat table summary)
## Limitations    (testnet mUSDC; no partial payments; DAO non-payment only affects score)
## Future         (mainnet USDC, NEAR Intents cross-chain payment, fractional invoices, DAO collateral)
```

**Rule:** if a feature isn't fully working at submission time, move it to Future. Never leave it under "What it does".

---

## 19. Build Plan (30 hours)

| Hours | Deliverable | Exit check |
|---|---|---|
| 0–1 | Toolchain pinned; testnet accounts funded; repo + CI skeleton | hello-world deployed |
| 1–3 | **Riskiest piece:** factor creates a keyless sub-account with global vault code | `list-keys` empty; vault `new` called. **If this fails at hour 3, switch to per-vault wasm.** |
| 3–5 | Sputnik DAO on testnet; FunctionCall proposal calling stub `accept_invoice` | executed via vote |
| 5–11 | Factor FR-1 to FR-8 | unit tests green |
| 11–13 | mUSDC + vault payment / withdraw | `t_happy_path` green |
| 13–15 | Security tests T1–T12 | all green |
| 15–17 | `make deploy dao demo`; `.env.example`; deployed addresses | runs from a fresh clone |
| 17–23 | Web UI, 3 screens | works against testnet |
| 23–26 | README (template §18), architecture image, events | every claim verified |
| 26–28 | Video (script §17) + X post | published |
| 28–29 | Fresh-clone dry run on another machine or container | `make demo` passes |
| 29–30 | **Submit (target 22:00 UTC, 14 Nov)** | form confirmation |

**If behind schedule, cut in this order:** UI polish, then filters, then the proposal generator (FR-11), then the UI itself (ship the CLI and a terminal video). **Never cut** the tests, `make demo` or the README.

---

## 20. Risks & Fallbacks
| Risk | Likelihood | Fallback |
|---|---|---|
| Global contract tooling breaks | Medium | Per-vault wasm deploy (~1.5–2 NEAR each on testnet) |
| Sputnik proposal flow slow to wire | Medium | Integration test with sputnikdao2 wasm in sandbox; demo can call accept from a plain DAO-stand-in account, labelled honestly |
| Testnet RPC flaky during recording | Low-Med | Record the terminal demo early (hour 17) as a backup |
| UI not finished | Medium | CLI + terminal video; the contracts are the product |
| Out of time | — | Follow the cut order in §19 |

---

## 21. Judge Q&A Prep
| Likely question (and who'd ask it) | Answer |
|---|---|
| "Who enforces the receivable?" (Haseeb) | The debtor is a DAO that acknowledged the debt by onchain vote. Its treasury and repayment history are public, so buyers price that risk. Non-payment is public and permanent in the payer score. Future: DAO-posted collateral. |
| "Why does this need NEAR?" (Illia) | The NFT owns a *named, keyless account*: payable by name, controlled only by code, created cheaply with global contracts. On EVM that needs ERC-6551 plus a registry. |
| "Can't someone fake a good payer history?" (Eddy) | The score is weighted by volume and account age, and the treasury balance is shown live. Tiny self-dealing histories score low. |
| "Is there a real market?" (Avichal) | DAOs pay contributors constantly and payout delays are normal. Factoring is a large TradFi category; this brings it to onchain organizations whose credit is checkable. |
| "Business model?" (Kain) | Protocol fee (e.g. 0.5%) on each sale; later, payer-score data and APIs for treasuries. |
| "What if the DAO pays before anyone buys?" | Then the contributor simply withdraws. FACTOR still works as a payable-by-name invoice with a receipt. |

---

## 22. Success Metrics

**Hackathon**
- Automated screen passes (fresh clone → `make test` + `make demo`).
- 12/12 security and integration tests green.
- Video ≤ 2:30 with the withdraw-fails/withdraw-succeeds moment.
- Submitted ≥ 2h before the deadline.

**Post-hackathon (sustainability story)**
- 3 real NEAR DAOs accept at least one invoice on mainnet.
- $10K of invoices factored in the first month.
- Trezu integration ("Pay invoice" button).

---

## 23. Submission Package

### Form description (~850 chars)
> FACTOR lets DAO contributors get paid today instead of waiting weeks. A contributor creates an invoice addressed to a Sputnik DAO; FACTOR mints an NFT that owns its own keyless NEAR sub-account (inv-42.factor.near), deployed via a global contract. The DAO approves the debt by onchain vote, a FunctionCall proposal calling accept_invoice. The contributor sells the invoice NFT at a discount for instant cash. The DAO later pays the vault's human-readable address, and whoever holds the NFT withdraws; the old owner can't. Every DAO builds a public payer score (on-time history, volume, treasury balance) shown in dynamic NFT metadata, so buyers price real, observable credit risk. Why NEAR: named sub-accounts plus keyless contract accounts give token-bound accounts natively. One command: make demo.

### X post
> Built FACTOR at NEAR HACKATHON 🧾⚡
> DAO contributors wait weeks to get paid. Now the DAO approves your invoice onchain, it becomes an NFT that owns its own NEAR account, and you sell it for cash today. Whoever holds the NFT gets paid.
> @NEARProtocol #NEARHackathon
> [video]

### Checklist
- [ ] Public repo, code written since the announcement
- [ ] README per §18, deployed addresses, tx links
- [ ] `make test` and `make demo` pass from a fresh clone
- [ ] X post with video, tags @NEARProtocol and #NEARHackathon
- [ ] Form: name, team lead, email, team size, **both tracks**, NEAR wallet address, repo link, X link, description

---

## 24. Future Roadmap (sustainability)
1. **Mainnet + native USDC**, same code.
2. **Pay from any chain:** the DAO pays the vault via NEAR Intents from Ethereum, Solana or elsewhere.
3. **Trezu integration:** "Approve invoice" and "Pay invoice" buttons inside DAO treasury tooling.
4. **Fractional invoices:** split one invoice across many buyers.
5. **DAO collateral / credit lines:** DAOs post collateral to earn better terms.
6. **Agent buyers:** AI agents that scan listings and buy within risk limits (an AI Swarms/Agent Economy extension).
7. **Protocol fee** on sales, plus payer-score APIs as a data product.
