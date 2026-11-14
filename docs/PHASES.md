# FACTOR: 6 Build Phases (Backend → Frontend) + Copy-Paste Prompts

**How to use this file**
1. Create an empty GitHub repo `factor` and clone it.
2. Put `FACTOR_PRD_full.md` in the repo root as `docs/PRD.md`.
3. Open your AI coding agent (Claude Code, Cursor, Codex…) in the repo.
4. Paste **the Master Context Prompt once**, then paste **one phase prompt at a time**, in order.
5. Don't start the next phase until the current phase's **exit check** passes. Commit after each phase.

| Phase | Name | Layer | Hours (target) | Exit check |
|---|---|---|---|---|
| 1 | Setup + riskiest spike | Backend | 0–3 | Factory creates a keyless sub-account running global vault code on testnet |
| 2 | Token + Vault contracts | Backend | 3–6 | mUSDC + vault unit tests green; fake payment rejected |
| 3 | Factor core contract | Backend | 6–12 | Full invoice lifecycle unit tests green |
| 4 | Sputnik DAO + integration & security tests | Backend | 12–16 | 12 sandbox integration tests green (incl. real Sputnik vote) |
| 5 | Testnet deploy + `make demo` | Backend/DevEx | 16–19 | `make deploy dao demo` prints 8 steps + DEMO PASSED |
| 6 | Frontend + README + submission assets | Frontend | 19–28 | UI works on testnet; README verified; fresh-clone run passes |

---

## Official docs the agent must use (referenced in every prompt)

| Topic | URL |
|---|---|
| Hackathon rules | https://nearhackathon.com/ |
| NEAR docs home / LLM index | https://docs.near.org · https://docs.near.org/llms.txt |
| Factory with global contract, **keyless sub-accounts** (the exact pattern we need) | https://docs.near.org/smart-contracts/tutorials/factories/factory |
| Global contracts (NEP-591) | https://docs.near.org/smart-contracts/global-contracts · https://docs.near.org/tutorials/examples/global-contracts |
| Actions (create account, transfer, function call) | https://docs.near.org/smart-contracts/anatomy/actions |
| near-sdk-rs API | https://docs.rs/near-sdk · https://github.com/near/near-sdk-rs |
| cargo-near | https://github.com/near/cargo-near |
| near-cli-rs | https://github.com/near/near-cli-rs |
| near-workspaces-rs (sandbox tests) | https://github.com/near/near-workspaces-rs |
| NEP-171 NFT core / NEP-177 metadata / NEP-181 enumeration | https://nomicon.io/Standards/Tokens/NonFungibleToken/Core |
| NEP-141 FT / NEP-145 storage | https://nomicon.io/Standards/Tokens/FungibleToken/Core · https://nomicon.io/Standards/StorageManagement |
| NEP-297 events | https://nomicon.io/Standards/EventsFormat |
| near-contract-standards crate (NFT/FT impls) | https://docs.rs/near-contract-standards |
| Sputnik DAO v2 contracts | https://github.com/near-daos/sputnik-dao-contract |
| Wallet Selector | https://github.com/near/wallet-selector |
| near-api-js | https://github.com/near/near-api-js |

---

## MASTER CONTEXT PROMPT (paste once, first)

```text
You are my senior NEAR Protocol engineer. We are building FACTOR for the NEAR Hackathon (https://nearhackathon.com/). Final deadline: 14 Nov 2026 23:59 UTC; we target submission by 22:00 UTC. Tracks: "NFTs That Do Something" and "Social Money".

Read docs/PRD.md completely before doing anything. It is the source of truth for scope, contract methods, data models, security rules (threats T1–T12), tests, demo output, and README format.

What FACTOR is: a contributor creates an invoice addressed to a Sputnik DAO. The factor contract mints a NEP-171 NFT and creates a KEYLESS sub-account vault (inv-<id>.<factor>) running a GLOBAL contract. The DAO accepts the invoice via a Sputnik FunctionCall proposal. The invoice NFT can be listed/bought at a discount. The DAO pays mUSDC (NEP-141) to the vault via ft_transfer_call → invoice PAID. Only the current NFT owner can withdraw. Per-DAO payer stats are public.

Hard rules for the entire project:
1. Use official NEAR docs and crates only. Before writing NEAR-specific code, consult these and follow their current APIs (do not rely on memory of old SDK versions):
   - https://docs.near.org/llms.txt (index)
   - https://docs.near.org/smart-contracts/tutorials/factories/factory (factory + global contract + keyless sub-accounts)
   - https://docs.near.org/smart-contracts/global-contracts
   - https://docs.rs/near-sdk (v5.x, enable the `global-contracts` feature if required)
   - https://docs.rs/near-contract-standards
   - https://github.com/near/near-workspaces-rs
   - https://github.com/near-daos/sputnik-dao-contract
   - NEP specs on nomicon.io (171, 177, 181, 141, 145, 297)
   If an API in the PRD doesn't match the current SDK, follow the SDK and note the change in docs/DECISIONS.md.
2. Rust contracts, near-sdk 5.x, built with cargo-near. Pin all versions (rust-toolchain.toml, Cargo.lock committed).
3. Testnet only. No mainnet funds, no API keys, no LLMs required to run anything.
4. Every security requirement in PRD §13 must be implemented AND tested.
5. Never claim a feature in README that is not implemented and tested. Unfinished items go under "Future".
6. Small commits with clear messages after each working step.
7. If blocked >30 minutes on something, stop, explain the blocker, and propose the fallback from PRD §20.
8. At the end of each phase, print: what was done, files changed, commands to verify, and the exit-check result.

Repo layout to create:
factor/
├─ contracts/{factor,vault,musdc}/
├─ tests/                 (near-workspaces integration tests)
├─ scripts/               (setup, deploy, dao, demo)
├─ web/                   (Next.js frontend, phase 6)
├─ docs/{PRD.md,DECISIONS.md}
├─ Makefile, rust-toolchain.toml, .env.example, README.md
└─ .github/workflows/ci.yml

Acknowledge you have read the PRD, list any ambiguities you see, then wait for the Phase 1 prompt.
```

---

## PHASE 1: Setup + Riskiest Spike (hours 0–3)

**Goal:** prove the hardest piece first. A factory contract creates a **keyless sub-account** running the vault as a **global contract**.

```text
PHASE 1 — Setup + riskiest spike.

Read: https://docs.near.org/smart-contracts/tutorials/factories/factory and https://docs.near.org/smart-contracts/global-contracts and https://docs.near.org/tutorials/examples/global-contracts. Follow that factory example's current code pattern exactly (create_account → transfer → use global contract → function_call init, with NO access keys added).

Tasks:
1. Create the repo layout from the master prompt. Add rust-toolchain.toml, a Cargo workspace for contracts/*, .gitignore, .env.example (NEAR_ENV=testnet, MASTER_ACCOUNT, MASTER_PRIVATE_KEY), and docs/DECISIONS.md.
2. Write a minimal `vault` contract: #[init] new(factory, stablecoin, invoice_id, amount_due) and a view get_config(). No other logic yet.
3. Write a minimal `factor` spike: init with the vault global contract reference (by hash; document hash vs account-id choice in DECISIONS.md), and a payable method spike_create(id) that creates `inv-<id>.<current_account>` with the global vault code, calls vault.new, adds NO keys, refunds excess deposit.
4. scripts/deploy_spike.sh using near-cli-rs: build both with cargo-near, deploy the vault wasm as a global contract (deploy-as-global), deploy factor, call spike_create(1), then print `near account list-keys inv-1.<factor>` (must be empty) and call get_config on the vault.
5. Makefile targets: build, spike.
6. Record measured storage/deposit cost of one vault in DECISIONS.md.

FALLBACK (only if global contracts fail after ~2h): factor stores the vault wasm bytes and uses deploy_contract on the sub-account. Document it in DECISIONS.md and continue.

Exit check: on testnet, inv-1.<factor> exists, has 0 access keys, and get_config returns the init values. Show me the commands and tx hashes.
```

**✅ Exit check:** the vault account shows 0 keys, `get_config` works, and the tx hashes are printed.

---

## PHASE 2: mUSDC + Vault Contracts (hours 3–6)

```text
PHASE 2 — Token + Vault contracts. Follow PRD §11.2, §11.3, §13 (T1, T8, T9).

Read: https://docs.rs/near-contract-standards (fungible_token module), NEP-141 + NEP-145 on nomicon.io, and near-sdk cross-contract call docs on docs.near.org.

Tasks:
1. contracts/musdc: standard NEP-141 + NEP-145 using near-contract-standards. 6 decimals, name "Mock USDC", symbol "mUSDC". Add `mint(account_id, amount)` open faucet clearly commented "TESTNET ONLY". Emit standard FT events.
2. contracts/vault (complete):
   - state: factory, stablecoin, invoice_id, amount_due, paid: bool
   - ft_on_transfer(sender_id, amount, msg) -> PromiseOrValue<U128>:
       require predecessor == stablecoin (else panic "ERR_WRONG_TOKEN")
       if paid → return full amount (refund)
       if amount < amount_due → return full amount (no partial payments)
       else paid = true; call factory.mark_paid(invoice_id, amount_due); return (amount - amount_due) as refund
   - withdraw_to(to): require predecessor == factory; ft_transfer full balance (query via own tracked amount = amount_due when paid) with 1 yoctoNEAR; return Promise so factor can attach a callback.
   - views: get_config(), is_paid()
   - NO methods that add keys or redeploy code.
3. Unit tests for vault: fake caller rejected (T1), underpay refunded (T8), double pay refunded (T9), excess refunded, withdraw_to only factory.
4. Unit tests for musdc: mint, transfer, storage_deposit.
5. Update Makefile: `make test` runs all unit tests.

Exit check: `make build && make test` green. Paste test output.
```

---

## PHASE 3: Factor Core Contract (hours 6–12)

```text
PHASE 3 — Factor core contract. Implement PRD §8 FR-1..FR-8, §11.1, §11.4, §12 exactly.

Read: https://docs.rs/near-contract-standards (non_fungible_token: core, metadata, enumeration), NEP-171/177/181 and NEP-297 on nomicon.io, near-sdk promises/callbacks docs on docs.near.org.

Tasks:
1. Replace the spike with the real contracts/factor:
   - NEP-171 + NEP-177 + NEP-181 via near-contract-standards NonFungibleToken.
   - State per PRD §12: invoices LookupMap<TokenId, Invoice>, listings IterableMap, payer_stats LookupMap<AccountId, PayerStats>, next_id, stablecoin, vault global code ref.
   - create_invoice(payer_dao, amount, due_ts, memo) payable: create keyless vault (Phase 1 pattern) → init vault → musdc.storage_deposit for the vault (paid from deposit) → mint NFT to caller → status Draft → emit invoice_created → refund unused deposit. Require deposit ≥ computed CREATE_COST.
   - accept_invoice(id): require predecessor == payer_dao (T3); Draft→Accepted; stats.accepted += 1; first_seen_ts if new; emit.
   - list(id, price) / delist(id): owner only, 1 yocto, status Accepted, !pending.
   - buy(id) payable: buyer != owner, deposit ≥ price; internal NFT transfer; pay seller; refund excess; clear listing; emit invoice_sold.
   - mark_paid(id, amount): require predecessor == invoice.vault (T2); Accepted→Paid; on-time vs late by due_ts; volume += amount; clear listing; emit.
   - withdraw(id): owner only, 1 yocto, status Paid, !pending → pending=true → vault.withdraw_to(owner).then(on_withdraw(id, owner)).
   - on_withdraw #[private]: pending=false ALWAYS; if promise success → Settled + emit; else keep Paid (T6).
   - Override nft_transfer / nft_transfer_call to require !pending (T5) and clear listing (T12).
   - Views: invoice(id) -> InvoiceView (include derived is_overdue, is_listed, price), payer_stats(dao) -> view with payer_score per PRD §12 formula, listings(from,limit), invoices_by_owner(owner).
   - nft_token metadata: title "FACTOR Invoice #id", description with amount/due/payer/status/score; extra = JSON of the invoice view.
2. Gas: set explicit Tgas for every cross-contract call; all calls ≤ 300 Tgas.
3. Unit tests: state machine valid/invalid transitions, payer_score function edge cases, list/buy/refund math, pending blocks transfer/list/buy, listing cleared on transfer.
4. cargo clippy clean.

Exit check: `make build && make test` green, clippy clean. Paste output and a summary table of methods implemented vs PRD §11.1.
```

---

## PHASE 4: Sputnik DAO + Integration & Security Tests (hours 12–16)

```text
PHASE 4 — Sputnik DAO integration + sandbox integration tests. Implement PRD §16 fully.

Read: https://github.com/near/near-workspaces-rs and https://github.com/near-daos/sputnik-dao-contract (sputnikdao2: add_proposal, act_proposal, policy, FunctionCall proposal kind). IMPORTANT: check the act_proposal signature in the current sputnikdao2 source (newer versions take id, action, proposal, memo) and use whatever the source says.

Tasks:
1. Add the sputnikdao2 wasm to tests/res/ (build from source or download the released wasm from the repo; document source + commit hash in DECISIONS.md).
2. tests/ with near-workspaces sandbox, helper fixtures: deploy musdc, vault global code (or fallback), factor, a Sputnik DAO with 3 council members threshold 2, accounts alice/bob/carol.
3. Implement every test in PRD §16 table:
   t_happy_path, t_vault_has_no_keys, t_accept_only_dao, t_accept_via_sputnik_proposal (real proposal → 2 votes → executes factor.accept_invoice), t_fake_payment_rejected, t_mark_paid_only_vault, t_underpay_refunded, t_double_pay_refunded, t_old_owner_withdraw_fails, t_transfer_blocked_while_pending, t_withdraw_failure_unlocks, t_listing_cleared_on_transfer, t_late_payment_counts_late (fast-forward blocks), t_create_requires_deposit.
   For the DAO paying the vault: use a FunctionCall proposal on musdc.ft_transfer_call to the vault (deposit 1 yocto), and ensure the DAO is storage-registered and funded with mUSDC.
4. If pending-state tests need it, add a test-only feature flag in vault to force withdraw failure; gate behind cfg feature, never in release build.
5. `make test` runs unit + integration. Add .github/workflows/ci.yml running make build test.

Exit check: all integration tests green. Paste the summary. Any test that can't pass → explain and propose fix before moving on.
```

---

## PHASE 5: Testnet Deploy + `make demo` (hours 16–19). Backend complete.

```text
PHASE 5 — Testnet deployment + one-command demo. Implement PRD §15 and the exact demo output format in §15.

Read: https://github.com/near/near-cli-rs (non-interactive command syntax), Sputnik factory on testnet (sputnik-v2.testnet — verify the current factory account in sputnik-dao-contract README; if different, use the README's value and note it).

Tasks:
1. scripts/env.sh: loads .env, derives account names from MASTER_ACCOUNT (factor.<master>, musdc.<master>, alice.<master>, bob.<master>, carol.<master>, council members), helper `tx_link <hash>` → https://testnet.nearblocks.io/txns/<hash>.
2. scripts/deploy.sh: create sub-accounts (idempotent: skip if exist), deploy musdc, deploy vault as global contract, deploy + init factor. Write deployed addresses to deployments/testnet.json.
3. scripts/dao.sh: create a Sputnik DAO via the testnet factory with council [bob, council2, council3] threshold 2-of-3; storage-register DAO on musdc; mint 10,000 mUSDC to the DAO. Save DAO id to deployments/testnet.json.
4. scripts/demo.sh: run the 8 steps from PRD §15 exactly, printing "[n/8] …  tx: <hash>  🔗 <link>". Step 2 must be a real proposal + 2 votes. Step 6 must show Alice's withdraw failing with NotOwner (expected) without aborting the script. End with payer stats and "DEMO PASSED ✅", exit non-zero if any expected outcome differs.
5. Makefile: deploy, dao, demo, all (= build test deploy dao demo). Scripts must be idempotent and safe to re-run (new invoice id each run).
6. Run it on testnet. Paste the full demo output.

Exit check: `make deploy dao demo` passes from a clean state with only .env filled. Save the output to docs/demo-output.txt (it will be referenced by README).
```

---

## PHASE 6: Frontend + README + Submission Assets (hours 19–28)

```text
PHASE 6 — Frontend, README, and submission polish. Implement PRD §14, §17, §18, §23.

Read: https://github.com/near/wallet-selector (React setup, MyNearWallet + Meteor wallet modules for testnet) and https://github.com/near/near-api-js (view calls, function calls). Use deployments/testnet.json for contract ids.

Tasks:
1. web/: Next.js (App Router) + TypeScript + Tailwind + @near-wallet-selector. Network testnet.
   - "/" Marketplace: cards per PRD §14 (invoice #, payer DAO, payer score badge green≥75/amber 50–74/red<50, face, price, discount %, implied APR, days to due, DAO mUSDC treasury balance via musdc.ft_balance_of, Buy button). Filters: min APR, min score, due within N days.
   - "/new" Create invoice form (payer DAO, amount, due date, memo) → create_invoice with correct deposit → redirect. Includes "Ask your DAO to approve" button that copies Sputnik proposal JSON (FR-11) for accept_invoice and for paying via musdc.ft_transfer_call.
   - "/invoice/[id]" Detail: status timeline with tx links (from NEP-297 events via a simple indexer call to nearblocks API if available, else show statuses without links — do not fake links), vault panel ("0 access keys ✓" from a real RPC view_access_key_list call), payer card, owner actions List/Delist/Withdraw with disabled states + tooltips.
   - Toast with explorer link after every tx. No dead buttons.
   - Show a "Testnet demo — mUSDC is a mock token" banner.
   - Implied APR uses a fixed demo NEAR↔mUSDC rate constant; label it in UI and README.
2. `make web` runs it locally; add web build to CI.
3. README.md exactly following PRD §18: what it does (each bullet with a real testnet tx link from docs/demo-output.txt), quick start, deployed addresses, architecture diagram (Mermaid), security table summary, limitations, Future. Verify every claim maps to a passing test or tx; remove anything that doesn't.
4. docs/SUBMISSION.md: final form description (≤2000 chars, from PRD §23, updated with real details), X post text, video script (PRD §17) with timestamps.
5. Fresh-clone check: in a temp dir, clone the repo, cp .env, run `make build test` and `make demo`. Fix anything that breaks. Report results.

Exit check: UI works end-to-end on testnet with a real wallet; README verified; fresh-clone run passes. Then print the final "YOUR MANUAL STEPS" checklist.
```

---

## Optional: one-shot prompt (if you want the agent to run all phases itself)

```text
Execute Phases 1→6 from docs/PHASES.md sequentially, following the MASTER CONTEXT rules. After each phase, run its exit check; do not proceed until it passes (use PRD §20 fallbacks if blocked). Commit after each phase with message "phase N: <name>". At the end, output: test summary, demo output, deployed addresses, README claim verification table, and the list of manual steps I must do.
```
(Save this file as `docs/PHASES.md` in the repo for this to work.)

---

## ✋ YOUR MANUAL STEPS (the agent can't do these for you)

### Before Phase 1 (10 min)
- [ ] Install Rust (`rustup`) and Node 20+. The agent can install `cargo-near` and `near-cli-rs`, but approve those installs.
- [ ] Create a **testnet account** (e.g. `yourname-factor.testnet`) using MyNearWallet / Meteor or `near account create-account sponsor-by-faucet-service`.
- [ ] Fund it with enough testnet NEAR (faucet). You need roughly 30–50 testnet NEAR for global contract deploy, sub-accounts, the DAO and repeated demos. If the faucet is limited, use several faucet requests or ask in NEAR dev Telegram/Discord.
- [ ] Export its private key into `.env` (`MASTER_ACCOUNT`, `MASTER_PRIVATE_KEY`). **Never commit `.env`.**
- [ ] Create the **public GitHub repo** and put the PRD at `docs/PRD.md`.

### During the build
- [ ] After each phase, **run the exit check yourself** (don't just trust the agent's word) and commit.
- [ ] At the Phase 1 exit, verify on https://testnet.nearblocks.io that the vault account has no keys.
- [ ] Phase 6: install a testnet wallet in your browser and click through the whole UI flow once yourself.

### Submission (hours 26–30)
- [ ] **Record the video** (≤ 2:30) following `docs/SUBMISSION.md`. Show the terminal `make demo` plus the UI, and make the "Alice withdraw ❌ / Carol withdraw ✅" moment clear.
- [ ] **Post on X:** say you built it at NEAR HACKATHON, tag **@NEARProtocol** and **#NEARHackathon**, attach the video/screenshots. Copy the post link.
- [ ] Make sure the repo is **public** and the README's top section is accurate.
- [ ] Have a **mainnet NEAR wallet address** ready (prizes are paid there).
- [ ] **Fill the submission form:** project name, team lead, email, team size, tracks (**NFTs That Do Something + Social Money**), NEAR wallet, repo link, X post link, description (20–2,000 chars, from `docs/SUBMISSION.md`).
- [ ] Submit by **22:00 UTC, 14 Nov** (03:30 IST, 15 Nov). The hard deadline is 23:59 UTC (05:29 IST, 15 Nov).
- [ ] If you made a mistake, use the site's **"Request changes"** page. Don't resubmit as a new project.

### Time reference for you (IST)
| Milestone | UTC | IST |
|---|---|---|
| Phase 1 done | 14 Nov ~00:00 | 14 Nov 05:30 |
| Backend done (Phase 5) | 14 Nov ~12:00 | 14 Nov 17:30 |
| Frontend done (Phase 6) | 14 Nov ~20:00 | 15 Nov 01:30 |
| **Submit** | **14 Nov 22:00** | **15 Nov 03:30** |
| Hard deadline | 14 Nov 23:59 | 15 Nov 05:29 |
