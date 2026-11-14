//! FACTOR — DAO-approved invoices as NFTs that own a keyless vault account.
//!
//! Flow: create_invoice (mint NFT + create keyless `inv-<id>.<factor>` on global vault code)
//!   -> accept_invoice (only the payer DAO, via a Sputnik FunctionCall proposal)
//!   -> list / buy (NEAR) -> DAO pays the vault in mUSDC -> vault calls mark_paid
//!   -> current NFT owner withdraws (pending lock + callback) -> SETTLED.
use near_contract_standards::non_fungible_token::{NonFungibleToken, TokenId};
use near_sdk::json_types::{Base58CryptoHash, U128, U64};
use near_sdk::serde_json::json;
use near_sdk::store::{IterableMap, LookupMap};
use near_sdk::{
    AccountId, BorshStorageKey, Gas, NearToken, PanicOnDefault, Promise, PromiseError, assert_one_yocto, env,
    ext_contract, near, require,
};

mod events;
mod nft;
pub mod score;
mod views;

pub use views::{InvoiceView, PayerStatsView};

/// NEAR sent to each new vault account (covers its ~250 bytes of state with margin).
pub const VAULT_FUND: NearToken = NearToken::from_millinear(10);
/// NEP-145 registration of the vault on mUSDC (FungibleToken min bound = 125 bytes).
pub const STABLECOIN_STORAGE_DEPOSIT: NearToken = NearToken::from_micronear(1_250);
pub const MAX_MEMO_LEN: usize = 280;

const GAS_VAULT_INIT: Gas = Gas::from_tgas(10);
const GAS_ON_VAULT_CREATED: Gas = Gas::from_tgas(25);
const GAS_STORAGE_DEPOSIT: Gas = Gas::from_tgas(10);
const GAS_WITHDRAW_TO: Gas = Gas::from_tgas(50);
const GAS_ON_WITHDRAW: Gas = Gas::from_tgas(15);

#[derive(BorshStorageKey)]
#[near]
pub(crate) enum StorageKey {
    OwnerById,
    TokensPerOwner,
    Invoices,
    Listings,
    PayerStats,
}

#[near(serializers = [borsh, json])]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Draft,
    Accepted,
    Paid,
    Settled,
}

#[near(serializers = [borsh])]
#[derive(Clone, Debug)]
pub struct Invoice {
    pub id: TokenId,
    pub creator: AccountId,
    pub payer_dao: AccountId,
    pub vault: AccountId,
    pub amount: u128,
    pub due_ts: u64,
    pub created_ts: u64,
    pub accepted_ts: Option<u64>,
    pub paid_ts: Option<u64>,
    pub settled_ts: Option<u64>,
    pub paid_on_time: Option<bool>,
    pub status: Status,
    pub pending: bool,
    pub memo: String,
}

#[near(serializers = [borsh])]
#[derive(Clone, Debug)]
pub struct Listing {
    pub seller: AccountId,
    pub price: u128, // yoctoNEAR
}

#[near(serializers = [borsh])]
#[derive(Clone, Debug, Default)]
pub struct PayerStats {
    pub accepted: u32,
    pub paid_on_time: u32,
    pub paid_late: u32,
    pub volume_paid: u128,
    pub first_seen_ts: u64,
}

#[ext_contract(ext_vault)]
#[allow(dead_code)]
trait Vault {
    fn withdraw_to(&mut self, to: AccountId) -> bool;
}

#[ext_contract(ext_ft_storage)]
#[allow(dead_code)]
trait StorageManagement {
    fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>);
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Factor {
    owner_id: AccountId,
    stablecoin: AccountId,
    vault_code_hash: Base58CryptoHash,
    next_id: u64,
    tokens: NonFungibleToken,
    invoices: LookupMap<TokenId, Invoice>,
    listings: IterableMap<TokenId, Listing>,
    payer_stats: LookupMap<AccountId, PayerStats>,
}

#[near]
impl Factor {
    #[init]
    pub fn new(owner_id: AccountId, stablecoin: AccountId, vault_code_hash: Base58CryptoHash) -> Self {
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::OwnerById,
                env::current_account_id(),
                None::<StorageKey>, // metadata is computed live in nft_token (FR-7), not stored
                Some(StorageKey::TokensPerOwner),
                None::<StorageKey>, // no approvals: only the owner can move an invoice NFT
            ),
            owner_id,
            stablecoin,
            vault_code_hash,
            next_id: 1,
            invoices: LookupMap::new(StorageKey::Invoices),
            listings: IterableMap::new(StorageKey::Listings),
            payer_stats: LookupMap::new(StorageKey::PayerStats),
        }
    }

    /// Owner only: point NEW invoices at new vault code. Existing vaults keep their code (D1).
    pub fn set_vault_code_hash(&mut self, vault_code_hash: Base58CryptoHash) {
        require!(env::predecessor_account_id() == self.owner_id, "ERR_NOT_CONTRACT_OWNER");
        self.vault_code_hash = vault_code_hash;
    }

    // ---------------------------------------------------------------- FR-1 create
    /// Creates `inv-<id>.<factor>` (keyless, global vault code), registers it on the stablecoin,
    /// mints the invoice NFT to the caller. Deposit must cover all storage (T11); excess refunded.
    #[payable]
    pub fn create_invoice(&mut self, payer_dao: AccountId, amount: U128, due_ts: U64, memo: String) -> TokenId {
        let creator = env::predecessor_account_id();
        let now = env::block_timestamp();
        require!(amount.0 > 0, "ERR_ZERO_AMOUNT");
        require!(due_ts.0 > now, "ERR_DUE_IN_PAST");
        require!(memo.len() <= MAX_MEMO_LEN, "ERR_MEMO_TOO_LONG");
        require!(payer_dao != creator, "ERR_SELF_INVOICE");

        let id_num = self.next_id;
        self.next_id += 1;
        let id: TokenId = id_num.to_string();
        let vault: AccountId = format!("inv-{id}.{}", env::current_account_id())
            .parse()
            .unwrap_or_else(|_| env::panic_str("ERR_VAULT_ID_TOO_LONG"));

        let storage_before = env::storage_usage();
        self.invoices.insert(
            id.clone(),
            Invoice {
                id: id.clone(),
                creator: creator.clone(),
                payer_dao: payer_dao.clone(),
                vault: vault.clone(),
                amount: amount.0,
                due_ts: due_ts.0,
                created_ts: now,
                accepted_ts: None,
                paid_ts: None,
                settled_ts: None,
                paid_on_time: None,
                status: Status::Draft,
                pending: false,
                memo,
            },
        );
        self.invoices.flush();
        self.tokens.internal_mint_with_refund(id.clone(), creator.clone(), None, None);
        near_contract_standards::non_fungible_token::events::NftMint {
            owner_id: &creator,
            token_ids: &[&id],
            memo: None,
        }
        .emit();
        let storage_used = env::storage_usage().saturating_sub(storage_before);
        let storage_cost = env::storage_byte_cost().saturating_mul(storage_used as u128);
        let required = storage_cost.saturating_add(VAULT_FUND).saturating_add(STABLECOIN_STORAGE_DEPOSIT);
        let deposit = env::attached_deposit();
        require!(
            deposit >= required,
            format!("ERR_DEPOSIT: attach at least {} yoctoNEAR", required.as_yoctonear())
        );
        let excess = deposit.saturating_sub(required);
        if !excess.is_zero() {
            let _ = Promise::new(creator.clone()).transfer(excess);
        }

        let _ = Promise::new(vault.clone())
            .create_account()
            .transfer(VAULT_FUND)
            .use_global_contract(self.vault_code_hash)
            .function_call(
                "new",
                json!({
                    "factory": env::current_account_id(),
                    "stablecoin": self.stablecoin,
                    "invoice_id": id,
                    "amount_due": amount,
                })
                .to_string()
                .into_bytes(),
                NearToken::from_yoctonear(0),
                GAS_VAULT_INIT,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(GAS_ON_VAULT_CREATED)
                    .on_vault_created(id.clone()),
            );

        events::emit(
            "invoice_created",
            json!({"id": id, "creator": creator, "payer_dao": payer_dao, "vault": vault,
                   "amount": amount, "due_ts": due_ts}),
        );
        id
    }

    #[private]
    pub fn on_vault_created(&mut self, id: TokenId, #[callback_result] r: Result<(), PromiseError>) -> bool {
        let inv = self.invoice_ref(&id).clone();
        if r.is_ok() {
            let _ = ext_ft_storage::ext(self.stablecoin.clone())
                .with_attached_deposit(STABLECOIN_STORAGE_DEPOSIT)
                .with_static_gas(GAS_STORAGE_DEPOSIT)
                .storage_deposit(Some(inv.vault), Some(true));
            return true;
        }
        // Vault creation failed (refund of VAULT_FUND came back to us): void the invoice.
        let owner = self.owner_of(&id);
        self.internal_burn(&id, &owner);
        self.invoices.remove(&id);
        let _ = Promise::new(inv.creator).transfer(VAULT_FUND.saturating_add(STABLECOIN_STORAGE_DEPOSIT));
        env::log_str(&format!("factor: vault creation failed, invoice #{id} voided"));
        false
    }

    // ---------------------------------------------------------------- FR-2 accept
    /// Only the payer DAO can accept (T3) — in practice via a passed Sputnik FunctionCall proposal.
    pub fn accept_invoice(&mut self, id: TokenId) {
        let mut inv = self.invoice_ref(&id).clone();
        require!(env::predecessor_account_id() == inv.payer_dao, "ERR_NOT_PAYER_DAO");
        require!(inv.status == Status::Draft, "ERR_BAD_STATUS");
        let now = env::block_timestamp();
        inv.status = Status::Accepted;
        inv.accepted_ts = Some(now);
        let mut stats = self.payer_stats.get(&inv.payer_dao).cloned().unwrap_or_default();
        if stats.first_seen_ts == 0 {
            stats.first_seen_ts = now;
        }
        stats.accepted += 1;
        self.payer_stats.insert(inv.payer_dao.clone(), stats);
        events::emit("invoice_accepted", json!({"id": id, "dao": inv.payer_dao}));
        self.invoices.insert(id, inv);
    }

    // ---------------------------------------------------------------- FR-3 market
    #[payable]
    pub fn list(&mut self, id: TokenId, price: U128) {
        assert_one_yocto();
        let inv = self.invoice_ref(&id);
        let owner = self.owner_of(&id);
        require!(env::predecessor_account_id() == owner, "ERR_NOT_OWNER");
        require!(inv.status == Status::Accepted, "ERR_BAD_STATUS");
        require!(!inv.pending, "ERR_PENDING");
        require!(price.0 > 0, "ERR_ZERO_PRICE");
        self.listings.insert(id.clone(), Listing { seller: owner, price: price.0 });
        events::emit("invoice_listed", json!({"id": id, "price": price}));
    }

    #[payable]
    pub fn delist(&mut self, id: TokenId) {
        assert_one_yocto();
        require!(env::predecessor_account_id() == self.owner_of(&id), "ERR_NOT_OWNER");
        require!(self.listings.remove(&id).is_some(), "ERR_NOT_LISTED");
        events::emit("invoice_delisted", json!({"id": id}));
    }

    /// Buy a listed invoice with NEAR. Seller gets `price`, buyer gets the NFT, excess refunded.
    #[payable]
    pub fn buy(&mut self, id: TokenId) {
        let buyer = env::predecessor_account_id();
        let listing = self.listings.get(&id).cloned().unwrap_or_else(|| env::panic_str("ERR_NOT_LISTED"));
        let inv = self.invoice_ref(&id);
        require!(!inv.pending, "ERR_PENDING");
        require!(inv.status == Status::Accepted, "ERR_BAD_STATUS");
        let owner = self.owner_of(&id);
        require!(owner == listing.seller, "ERR_STALE_LISTING");
        require!(buyer != owner, "ERR_OWN_LISTING");
        let deposit = env::attached_deposit().as_yoctonear();
        require!(deposit >= listing.price, "ERR_PRICE_TOO_LOW");

        self.listings.remove(&id);
        self.tokens.internal_transfer(&owner, &buyer, &id, None, Some("FACTOR buy".into()));
        let _ = Promise::new(owner.clone()).transfer(NearToken::from_yoctonear(listing.price));
        if deposit > listing.price {
            let _ = Promise::new(buyer.clone()).transfer(NearToken::from_yoctonear(deposit - listing.price));
        }
        events::emit(
            "invoice_sold",
            json!({"id": id, "from": owner, "to": buyer, "price": U128(listing.price)}),
        );
    }

    // ---------------------------------------------------------------- FR-4 paid
    /// Called by the invoice's own vault after a valid mUSDC payment (T2).
    /// Panicking here makes the vault refund the payment (D6).
    pub fn mark_paid(&mut self, id: TokenId, amount: U128) {
        let mut inv = self.invoice_ref(&id).clone();
        require!(env::predecessor_account_id() == inv.vault, "ERR_ONLY_VAULT");
        require!(inv.status == Status::Accepted, "ERR_BAD_STATUS");
        require!(amount.0 == inv.amount, "ERR_AMOUNT_MISMATCH");
        let now = env::block_timestamp();
        let on_time = now <= inv.due_ts;
        inv.status = Status::Paid;
        inv.paid_ts = Some(now);
        inv.paid_on_time = Some(on_time);
        let mut stats = self.payer_stats.get(&inv.payer_dao).cloned().unwrap_or_default();
        if on_time {
            stats.paid_on_time += 1;
        } else {
            stats.paid_late += 1;
        }
        stats.volume_paid = stats.volume_paid.saturating_add(amount.0);
        self.payer_stats.insert(inv.payer_dao.clone(), stats);
        self.listings.remove(&id); // T12
        events::emit("invoice_paid", json!({"id": id, "amount": amount, "on_time": on_time}));
        self.invoices.insert(id, inv);
    }

    // ---------------------------------------------------------------- FR-5 withdraw
    /// Current NFT owner pulls the payment from the vault (T4). Locks the invoice until the
    /// callback runs (T5); the callback always unlocks (T6).
    #[payable]
    pub fn withdraw(&mut self, id: TokenId) -> Promise {
        assert_one_yocto();
        let owner = self.owner_of(&id);
        require!(env::predecessor_account_id() == owner, "ERR_NOT_OWNER");
        let mut inv = self.invoice_ref(&id).clone();
        require!(inv.status == Status::Paid, "ERR_NOT_PAID");
        require!(!inv.pending, "ERR_PENDING");
        inv.pending = true;
        let vault = inv.vault.clone();
        self.invoices.insert(id.clone(), inv);
        ext_vault::ext(vault)
            .with_static_gas(GAS_WITHDRAW_TO)
            .withdraw_to(owner.clone())
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_WITHDRAW).on_withdraw(id, owner))
    }

    #[private]
    pub fn on_withdraw(&mut self, id: TokenId, owner: AccountId, #[callback_result] r: Result<bool, PromiseError>) -> bool {
        let mut inv = self.invoice_ref(&id).clone();
        inv.pending = false;
        let ok = matches!(r, Ok(true));
        if ok {
            inv.status = Status::Settled;
            inv.settled_ts = Some(env::block_timestamp());
            events::emit("invoice_settled", json!({"id": id, "to": owner, "amount": U128(inv.amount)}));
        } else {
            env::log_str(&format!("factor: withdraw of #{id} failed; invoice stays PAID and unlocked"));
        }
        self.invoices.insert(id, inv);
        ok
    }

    // ---------------------------------------------------------------- helpers
    pub(crate) fn invoice_ref(&self, id: &TokenId) -> &Invoice {
        self.invoices.get(id).unwrap_or_else(|| env::panic_str("ERR_NO_INVOICE"))
    }

    pub(crate) fn owner_of(&self, id: &TokenId) -> AccountId {
        self.tokens.owner_by_id.get(id).unwrap_or_else(|| env::panic_str("ERR_NO_TOKEN"))
    }

    fn internal_burn(&mut self, id: &TokenId, owner: &AccountId) {
        self.tokens.owner_by_id.remove(id);
        if let Some(per_owner) = self.tokens.tokens_per_owner.as_mut()
            && let Some(mut set) = per_owner.get(owner)
        {
            set.remove(id);
            if set.is_empty() {
                per_owner.remove(owner);
            } else {
                per_owner.insert(owner, &set);
            }
        }
    }
}

#[cfg(test)]
mod tests;
