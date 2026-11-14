# Feedback for the NEAR team: global contracts are referenced by a bare hash

Written by the FACTOR team for the judges and for the NEAR protocol and tooling teams. Everything under "What we observed" happened while building FACTOR on testnet. Everything under "Proposed solution" is a proposal, not something we shipped in the protocol. Section 7 lists exactly which parts we did ship.

## 1. Summary

NEAR global contracts are the right primitive for what FACTOR does: every invoice gets its own keyless vault account, and all vaults run one shared copy of the code, referenced by its hash. That is cheap per vault and removes upgrade keys.

The pain point: **a global contract is referenced by a bare 32-byte hash, and nothing in the platform helps you know that the hash is real, what code it is, or what publishing it costs.** We hit three consequences while building:

1. A script pointed our factor contract at a hash that was never published. Every `create_invoice` call then failed with `GlobalContractDoesNotExist` (reported by the wallet, code -1432). That is a late, opaque error, and the person who sees it is the end user.
2. A rebuild of the same source gave a different hash, and publishing it burns about 11 NEAR for a 113 KB contract. It is easy to burn funds by accident.
3. The hash says nothing about its source. A judge or user cannot tell what code the vaults run without trusting us.

Our fix is small and shipped (existence preflight, no silent re-publish, refund on failure). The proper fix has to live in the platform, and we describe it in section 5.

## 2. What we observed

| # | Observation | Where it shows up in the repo |
|---|---|---|
| O1 | Pointing the factor at an unpublished hash (`HHZS…`) made `create_invoice` fail for the user with `GlobalContractDoesNotExist`. The factor accepted the hash without any check, because `set_vault_code_hash` stores whatever it is given. | `docs/DECISIONS.md` D12, `contracts/factor/src/lib.rs` (`set_vault_code_hash`) |
| O2 | The global publish had failed earlier, but our deploy script hid the failure. Nothing in the flow forced us to confirm that the hash existed before using it. That part was our own bug. | `scripts/deploy.sh` (fixed) |
| O3 | cargo-near builds were not byte-reproducible between rebuilds in our setup, so the same source gave a different hash each time. | D12 |
| O4 | Publishing a global contract burned about 11 NEAR for our 113 KB vault (about 10 NEAR per 100 KB). We found this out by paying it. | D12 |
| O5 | The common sandbox test library (near-workspaces 0.24) had no global-contract deploy action, so the headline feature could not be tested with it. We switched to near-sandbox plus near-api. | `docs/DECISIONS.md` D9 |
| O6 | We had to write the existence check ourselves with the raw RPC call `view_global_contract_code`. | `scripts/env.sh` (`global_exists`) |
| O7 | Public RPC rate limiting (HTTP 429) looked like a CORS error in the browser, so the app could not tell "rate limited" from "broken". We worked around it with an RPC fallback list and a cache. | `web/lib/near.ts` |

## 3. Reproduction

1. Build the vault wasm and compute its hash.
2. Publish it as a global contract, but let the publish fail or skip it (for example, an underfunded account with errors hidden).
3. Call `set_vault_code_hash` on a contract that uses `Promise::use_global_contract` with that hash. It succeeds.
4. Call the function that creates the account. The failure only appears now: `GlobalContractDoesNotExist`.

In steps 2 and 3 nothing warned us.

## 4. The real pain point

This is not one bug. It is a missing lifecycle around the hash. The platform lets you use a global contract but gives you no help with three questions:

| Question | What exists today | Gap |
|---|---|---|
| Does this hash exist right now? | An RPC call (`view_global_contract_code`) | No simple check from inside a contract or from the standard publish flow. We hand-rolled it. |
| What code is behind this hash? | The wasm bytes, if you download them | No link from the hash to a source repository or build recipe. Reproducible builds exist but are opt-in and slower. |
| What will publishing cost? | Learned by paying | No estimate before the transaction. Every non-reproducible rebuild is a new hash and a new cost. |

For an invoice product the consequences are concrete. A wrong hash blocks every new invoice. An unverifiable hash means buyers must trust that the vault really has no keys and no backdoor.

## 5. What could have been done: proposed solution

These are proposals for the NEAR core and tooling teams. We did not implement them in the protocol.

1. **Fail early on a missing hash.** Validate that the global code exists when the action is built or when `use_global_contract` is called, and return a clear error that names the hash. Better still, let a contract ask "does this hash exist?" so `set_vault_code_hash` can refuse a bad value.
2. **Publish with a cost estimate.** A dry-run or estimate option on the publish command that prints the burn in NEAR before signing, and refuses by default if the account cannot cover it.
3. **Make builds reproducible by default.** Make the reproducible build the default path for a global contract, or warn loudly when a hash comes from a non-reproducible build.
4. **Link a hash to its source.** Publishing a global contract could record NEP-330 style source metadata (repo, commit, build image) next to the hash, so explorers can show "this hash was built from this commit" and anyone can reproduce it.
5. **Test support.** Add global-contract deploy and use actions to the main sandbox test library so the feature can be tested without dropping to lower-level crates.
6. **RPC errors with proper headers.** Return CORS headers on error responses (including 429), so browser apps can tell "rate limited, retry later" from a real failure.

```
Today                                    Proposed
-----                                    --------
publish (cost unknown)                   publish with estimate -> shows burn, confirms
   |                                        |
set hash in contract (no check)          set hash in contract  -> checks it exists
   |                                        |
user calls create -> late error          user calls create     -> works, or a clear error
   |                                        |
"what code is this?" -> unknown          explorer shows hash   -> repo + commit
```

## 6. Plan to deliver the fix

| Step | Owner | Work | Done when |
|---|---|---|---|
| 1 | FACTOR (us) | Existence preflight in our deploy script, no silent re-publish, refund path in the factor | Shipped (section 7) |
| 2 | FACTOR (us) | Show a clear message in the web app before `create_invoice` if the factor's vault hash is not published, instead of a wallet failure | Not done. Next step on our side. |
| 3 | NEAR tooling | Cost estimate on global publish; global actions in the main sandbox test library | A released CLI or library version |
| 4 | NEAR core | Early, named error for a missing global hash; contract-side existence check | Behaviour in a protocol release |
| 5 | NEAR tooling | Source metadata on publish, shown by explorers | A hash page that links to a repo and commit |

Risks: an existence check inside a contract can race with a later removal of the code, so it must be treated as best effort and the use-time error must stay clear. Reproducible builds are slower, so the estimate matters more than forcing them.

How to measure success: a developer can publish, point a contract at the hash and get a clear error at the earliest possible step; a rebuild never costs funds without a visible estimate; and a third party can map a hash to a commit without asking the team.

## 7. What we shipped in FACTOR (mitigations)

| Item | Status |
|---|---|
| `scripts/deploy.sh` checks on chain that the vault hash exists before pointing the factor at it | Shipped |
| The script keeps the published hash unless `NEW_VAULT=1`, so it never burns about 11 NEAR silently | Shipped |
| The script exits with an error if a publish fails, instead of continuing | Shipped |
| `on_vault_created` voids the invoice and refunds the creator if the vault cannot be created | In the contract. We did not write a dedicated test for the missing-hash case. |
| The factor uses the published hash `64WtqCUfEVtuu4JZt18k58WdfkjCzH6sGvZA5RU6m7X1`. Invoice creation works on testnet again and the 8-step demo passes. | Verified |
| Reproducible build for a mainnet release (`cargo near build reproducible-wasm`) | Documented in D12, not used for the testnet deploy |
| RPC fallback list and cache in the web app | Shipped |

## 8. Secondary friction

| Area | Observation |
|---|---|
| Sputnik DAO | Voting on a proposal requires sending the full proposal body again in `act_proposal`, and `FunctionCall` arguments must be base64-encoded by hand. Easy to get wrong in scripts. |
| Sputnik DAO | There is no standard way for a contract to tell that a call came from an executed proposal, other than checking that the caller is the DAO account. FACTOR relies on that check and it works, but it is an implicit convention. |

## 9. What we did not verify

- We did not read the protocol source to confirm exactly where `GlobalContractDoesNotExist` is raised. We only saw the error and its effect for the user.
- The wallet showed the error for the `create_invoice` call. We did not capture the exact receipt tree of that failed call.
- We did not test whether a newer cargo-near release makes the build reproducible by default.
- The proposals in section 5 are our suggestions. The NEAR team may already have some of them planned, or may know constraints we do not.
