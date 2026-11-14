# FACTOR: submission package

> `make deploy dao demo` has run on testnet (see docs/demo-output.txt). Fill the remaining `<…>` placeholders yourself. Do not claim anything the demo output does not show.

## Form fields
- **Project name:** FACTOR
- **Tracks:** NFTs That Do Something + Social Money
- **Repo:** https://github.com/SamuelDharshi/factor (must be public)
- **X post link:** `<paste after posting>`
- **NEAR wallet for prizes:** `<your mainnet account>`

## Description (≈900 characters, limit 2,000)

FACTOR lets DAO contributors get paid today instead of waiting weeks. A contributor creates an invoice addressed to a Sputnik DAO; FACTOR mints an NFT and a keyless NEAR sub-account for it (inv-42.factor…), created from a global contract so every invoice shares one vault code hash. The DAO accepts the debt with a real onchain vote: a Sputnik FunctionCall proposal that calls accept_invoice. The contributor can sell the invoice NFT at a discount for instant cash. When the DAO pays the vault's human-readable address in a stablecoin, the invoice becomes PAID and whoever holds the NFT can withdraw; the old owner cannot. Every DAO builds a public payer score (on-time record, volume, history) shown in live NFT metadata and next to its treasury balance, so buyers can price real, observable credit risk. Why NEAR: named sub-accounts, keyless contract accounts and global contracts give token-bound accounts natively. An on-chain registry lets AI agents and robots register on NEAR with a public buying strategy. 23 sandbox tests cover 13 threats. One command reproduces the story on testnet: make demo.

## X post

Built FACTOR at NEAR HACKATHON 🧾⚡
DAO contributors wait weeks to get paid. Now the DAO approves your invoice onchain, it becomes an NFT that owns its own NEAR account, and you can sell it for cash today. Whoever holds the NFT gets paid.
@NEARProtocol #NEARHackathon
`<video / screenshots>`

## Video script (target 2:20; max 2:30)

| Time | Show | Say |
|---|---|---|
| 0:00–0:12 | Title card | "DAO contributors on NEAR do the work, then wait weeks to get paid. FACTOR fixes that." |
| 0:12–0:30 | `make demo` step 1: invoice created; vault line "access keys: 0 ✓" | "Alice invoices a Sputnik DAO for 500 mUSDC. FACTOR mints an NFT that owns its own NEAR account. It has no keys, so only code can move its money." |
| 0:30–0:50 | Step 2: proposal + 2 of 3 votes → executed | "The DAO approves the invoice by an onchain vote. That's a public debt acknowledgment." |
| 0:50–1:10 | UI marketplace card: APR, payer score, treasury; step 3–4 | "Alice lists it at a discount. Carol sees the implied APR, the DAO's score and treasury, and buys. Alice has cash today." |
| 1:10–1:30 | Step 5: DAO pays the vault address → PAID | "Later the DAO pays the invoice's address. It's just a name." |
| 1:30–1:50 | **Step 6 ❌ NotOwner, step 7 ✅ 500 received** | "Alice can't withdraw anymore. The money follows the NFT. Carol withdraws 500." |
| 1:50–2:10 | Step 8 payer score; architecture diagram from README | "Every DAO builds a public payer score. On NEAR this is native: named sub-accounts, keyless contracts, global contracts." |
| 2:10–2:25 | `DEMO PASSED ✅` | "One command reproduces everything on testnet. FACTOR: get paid by DAOs today." |

Record the terminal demo early as a backup in case testnet RPC is flaky.

## Checklist
- [ ] Repo is public; README status line updated and tx links added from `docs/demo-output.txt`
- [ ] `make build test` and `make deploy dao demo` pass from a fresh clone
- [ ] Video recorded (≤ 2:30), X post published with @NEARProtocol and #NEARHackathon
- [ ] Form submitted by 22:00 UTC 14 Nov (hard deadline 23:59 UTC). Mistakes: use "Request changes", do not resubmit.
