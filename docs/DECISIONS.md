# FACTOR — Engineering Decisions

Deviations from `docs/PRD.md` and the reasons for them. Newest at the bottom of each section.

## Toolchain (Phase 1)
| Item | Version | Why |
|---|---|---|
| Rust | 1.95.0 (`rust-toolchain.toml`) | Same as the `cargo near new` template; near-sdk 5.30 has `rust-version = 1.95` |
| near-sdk / near-contract-standards | =5.30.1 | Latest 5.x at build time; `global-contracts` feature enabled |
| cargo-near | 0.23.0 | Builds with `cargo near build non-reproducible-wasm --no-abi` |
| near-cli-rs | 0.31.1 | Every script uses `sign-with-plaintext-private-key` (no keychain) |
| Sandbox tests | near-sandbox =0.4.1 + near-api =0.8.7 | **PRD said near-workspaces.** The current `cargo near new` template uses near-sandbox + near-api, and near-workspaces 0.24 has no global-contract deploy action. near-api has `Contract::deploy_global_contract_code(..).as_hash()` and `use_global_hash`. |

## D1. Vault code is referenced **by hash**, not by account id
- **By hash** (`Promise::use_global_contract(hash)`): immutable. Nobody, including the factor owner, can ever change the code a vault runs. Existing invoices keep the exact code they were created with, which is the core trust property ("only this code can move the money").
- **By account id**: mutable, so the deployer could swap the code under every live vault. That's a rug vector, so we rejected it.
- Cost: an upgrade means deploying a new hash and calling `factor.set_vault_code_hash` (owner only), which applies to **new** invoices only. Old vaults stay on old code by design.

## D2. Vault creation pattern (exact factory-tutorial pattern)
`Promise::new("inv-<id>.<factor>").create_account().transfer(VAULT_FUND).use_global_contract(hash).function_call("new", …)`. **No `add_full_access_key` / `add_access_key` action**, so the account has 0 keys. The vault's `new` requires `predecessor == factory`, and the vault exposes no key or deploy methods.

## D3. Measured vault cost (Phase 1, sandbox; same protocol rules as testnet)
- Vault account `storage_usage` = **233 bytes** (account record + 32-byte global hash reference + state). That's under the 770-byte zero-balance-account limit, so it needs no storage staking.
- We still send `VAULT_FUND = 0.01 NEAR` so the vault can absorb state growth (the `paid` flag in Phase 2) without hitting the limit.
- Global code deploy (one-time, burnt): 10 NEAR per 100 KB. The final vault wasm is about 113 KB (90 KB of code, 22 KB of data, no custom sections; that is the near-sdk 5.x + serde_json baseline), so about **11.3 NEAR once**. `near contract deploy-as-global … as-global-hash` skips the deploy if that hash already exists.
- So per-invoice cost is 0.01 NEAR to the vault, plus the mUSDC storage deposit (0.00125 NEAR) and NFT/invoice storage on factor (Phase 3 measures it). That's far below the PRD's 0.5 NEAR budget.

## D4. Testnet spending
The global deploy burns NEAR on every new hash, so we deploy only the **final** vault code to testnet (Phase 5). The Phase 1 keyless/global exit check is proven in sandbox (`tests/tests/spike.rs`). `scripts/deploy_spike.sh` (`make spike`) reproduces it on testnet. Phase 5's real deploy shows the same property on `inv-<id>.factor.<master>` (`list-keys` is empty).

## D5. Demo accounts share the master key
alice, bob, carol, council2, council3, factor and musdc are sub-accounts of `MASTER_ACCOUNT`, created with the master's public key (derived from the secret key by `scripts/pubkey.py`). The only secret is `MASTER_PRIVATE_KEY` in `.env`, so a fresh clone can re-run everything. Vaults are the exception: they never get keys.

## D6. Vault payment is only committed after the factor accepts `mark_paid` (new threat T13)
PRD §11.2 sets `paid = true` and returns the refund synchronously. If `factor.mark_paid` then fails (for example the DAO pays a DRAFT invoice before accepting it), the funds would sit in a vault the factor considers unpaid, and nobody could ever withdraw them. Instead, `ft_on_transfer` returns `mark_paid(..).then(on_mark_paid(amount))`. On success it refunds `amount - due`. On failure it resets `paid = false` and refunds `amount` in full. Unit test: `t13_rejected_mark_paid_unwinds_and_refunds`.

## D7. Vault `withdraw_to` resolves to a bool
`withdraw_to` sets `withdrawn = true`, calls `ft_transfer(to, amount_due)` (1 yocto), then `on_transferred`. On failure that resets `withdrawn` and returns `false` instead of panicking, so the reset isn't rolled back. The factor's `on_withdraw` treats "promise failed OR returned false" as failure (T6). A real failure is easy to trigger without any test-only flag: the receiver isn't storage-registered on mUSDC, so `ft_transfer` panics. That's what `t_withdraw_failure_unlocks` uses. No cfg feature is needed, so the release build can't carry a test backdoor.
The vault sends `amount_due`, not "its balance". A vault can't read its own NEP-141 balance synchronously, and every excess payment is already refunded. mUSDC sent with plain `ft_transfer` (not `_call`) is unrecoverable dust and is listed under Limitations.

## D8. Sputnik DAO wasm for tests
`tests/res/sputnikdao2.wasm` is built from https://github.com/near-daos/sputnik-dao-contract at commit `5434a59dec23ee0c49df8c8c4350c208a7e75b0b` (sputnikdao2 v2.3.1, 2026-05-28) with `cargo near build non-reproducible-wasm --no-abi` (rust 1.95; 439,510 bytes; sha256 `1b55b5a001bc25ca39de2bede8e91f7f893e0c665bc8513f3056c93cddc5dacf`). The current `act_proposal` signature is `(id, action, proposal: ProposalKind, memo)`, and the tests use it. `new(config, policy)` with `policy = [bob, council2, council3]` builds the default policy (council role, ratio 1/2, which means 2 of 3 votes), and a proposal needs a 1 NEAR bond.

## D9. Sandbox tests use near-sandbox + near-api, not near-workspaces
See the toolchain table. Test fixtures live in `tests/src/lib.rs`, tests in `tests/tests/integration.rs`. Each test gets its own sandbox: real mUSDC, vault as global code by hash, factor, a real Sputnik DAO, plus accounts.

## D10. Testing "transfer while pending" (T5) without a test-only backdoor
A transaction can hold several function-call actions to the same contract, and they run in order inside one receipt. The test sends `withdraw` then `nft_transfer` as two actions in a single transaction. `withdraw` sets `pending = true`, then `nft_transfer` must abort with `ERR_PENDING`, and the whole transaction reverts. This tests the real pending path with no cfg feature and no test-only code in the contracts.

## D11. Tests added beyond the PRD §16 table
`t_dao_pays_via_sputnik_proposal` (the real payment path through governance), `t_excess_refunded`, `t_pay_before_accept_refunded` (T13), `t_listing_cleared_on_payment`, `t_create_refunds_excess_and_is_cheap`, `t_market_rules`, `t_nft_metadata_is_live`.

## D12. Global vault code: publish once, verify before use
Publishing a global contract burns ~11 NEAR per new hash, and cargo-near builds are not byte-reproducible across rebuilds, so a rebuilt vault has a different hash than the published one. A rebuild followed by `make deploy` once pointed the factor at a hash that was never published (the global deploy failed and the script swallowed the error), so `create_invoice` failed on testnet with `GlobalContractDoesNotExist`. Fixed by repointing the factor at the published hash `64Wt…` (the one invoice #1's vault runs). `scripts/deploy.sh` now checks `view_global_contract_code` on-chain, keeps the published hash unless `NEW_VAULT=1`, and exits with an error if a publish fails. For a mainnet release, build with `cargo near build reproducible-wasm` (docker) so the hash is verifiable from source.

## D13. Agent/robot registry (new contract)
`contracts/registry` lets AI agents and robots register on NEAR (one entry per account), pay for their own storage (refunded on deregister), and publish a strategy (min payer score, min implied APR, max price). The strategy is public metadata that the agent's own code is expected to follow; the contract does not enforce it, and agents trade on FACTOR by calling `factor.buy` from their own account. Entries can be paused, updated by the owner only, and deregistered.
