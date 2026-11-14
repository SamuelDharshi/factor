//! mUSDC: Mock USDC, a standard NEP-141 + NEP-145 token with 6 decimals.
//!
//! !!! TESTNET ONLY !!!  `mint` is an OPEN FAUCET: anyone can mint up to MAX_MINT to any
//! registered account. The token has no value and exists only to demo FACTOR.
use near_contract_standards::fungible_token::events::FtMint;
use near_contract_standards::fungible_token::metadata::{
    FT_METADATA_SPEC, FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenResolver,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    AccountId, BorshStorageKey, NearToken, PanicOnDefault, PromiseOrValue, log, near, require,
};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    FungibleToken,
    Metadata,
}

/// Max a single faucet call may mint (1,000,000 mUSDC); keeps total supply far from u128 overflow.
pub const MAX_MINT: u128 = 1_000_000 * 1_000_000;

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "Mock USDC".to_string(),
            symbol: "mUSDC".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 6,
        };
        Self {
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        }
    }

    /// TESTNET ONLY: open faucet. `account_id` must already be storage-registered (NEP-145).
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(amount.0 > 0 && amount.0 <= MAX_MINT, "ERR_MINT_AMOUNT");
        self.token.internal_deposit(&account_id, amount.0);
        FtMint {
            owner_id: &account_id,
            amount,
            memo: Some("testnet faucet"),
        }
        .emit();
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used, burned) = self
            .token
            .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned > 0 {
            log!("Account @{} burned {}", sender_id, burned);
        }
        used.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        self.token.internal_storage_unregister(force).is_some()
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::testing_env;

    fn ctx(pred: AccountId, deposit: NearToken) {
        testing_env!(
            VMContextBuilder::new()
                .current_account_id("musdc.testnet".parse().unwrap())
                .predecessor_account_id(pred)
                .attached_deposit(deposit)
                .build()
        );
    }

    fn registered() -> Contract {
        ctx(accounts(0), NearToken::from_yoctonear(0));
        let mut c = Contract::new();
        let min = c.storage_balance_bounds().min;
        for i in 0..3 {
            ctx(accounts(i), min);
            c.storage_deposit(None, None);
        }
        c
    }

    #[test]
    fn metadata_is_mock_usdc_6_decimals() {
        let c = registered();
        let m = c.ft_metadata();
        assert_eq!((m.name.as_str(), m.symbol.as_str(), m.decimals), ("Mock USDC", "mUSDC", 6));
    }

    #[test]
    fn storage_deposit_registers() {
        let c = registered();
        assert!(c.storage_balance_of(accounts(1)).is_some());
        assert!(c.storage_balance_of(accounts(4)).is_none());
    }

    #[test]
    fn mint_increases_balance_and_supply() {
        let mut c = registered();
        ctx(accounts(2), NearToken::from_yoctonear(0));
        c.mint(accounts(1), U128(500_000_000));
        assert_eq!(c.ft_balance_of(accounts(1)).0, 500_000_000);
        assert_eq!(c.ft_total_supply().0, 500_000_000);
    }

    #[test]
    #[should_panic(expected = "is not registered")]
    fn mint_to_unregistered_fails() {
        let mut c = registered();
        c.mint(accounts(4), U128(1));
    }

    #[test]
    #[should_panic(expected = "ERR_MINT_AMOUNT")]
    fn mint_cap() {
        let mut c = registered();
        c.mint(accounts(1), U128(MAX_MINT + 1));
    }

    #[test]
    fn transfer_moves_balance() {
        let mut c = registered();
        c.mint(accounts(1), U128(1_000));
        ctx(accounts(1), NearToken::from_yoctonear(1));
        c.ft_transfer(accounts(2), U128(400), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 600);
        assert_eq!(c.ft_balance_of(accounts(2)).0, 400);
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn transfer_requires_one_yocto() {
        let mut c = registered();
        c.mint(accounts(1), U128(1_000));
        ctx(accounts(1), NearToken::from_yoctonear(0));
        c.ft_transfer(accounts(2), U128(1), None);
    }
}
