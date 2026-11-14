//! FACTOR vault — one per invoice, deployed ONCE as a global contract (by code hash).
//!
//! Each invoice gets a keyless sub-account `inv-<id>.<factor>` that *uses* this global code.
//! The account has zero access keys and this contract exposes NO method that adds keys,
//! deploys code or deletes the account, so only this code — driven by the factor contract —
//! can ever move its funds.
//!
//! Payment: the DAO calls `musdc.ft_transfer_call(vault, amount)`; mUSDC calls `ft_on_transfer`.
//!
//! - only the whitelisted stablecoin may call it (T1)
//! - underpayment / second payment -> full refund (T8, T9)
//! - otherwise `factor.mark_paid(id, amount_due)`; on success refund the excess, on failure
//!   (e.g. invoice not yet ACCEPTED) undo `paid` and refund everything (T13).
//!
//! Withdraw: only the factor may call `withdraw_to` (it checks NFT ownership + pending lock).
use near_sdk::json_types::U128;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError, PromiseOrValue, env, ext_contract,
    near, require,
};

const GAS_MARK_PAID: Gas = Gas::from_tgas(20);
const GAS_ON_MARK_PAID: Gas = Gas::from_tgas(10);
const GAS_FT_TRANSFER: Gas = Gas::from_tgas(15);
const GAS_ON_TRANSFERRED: Gas = Gas::from_tgas(10);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

#[ext_contract(ext_factor)]
#[allow(dead_code)]
trait Factor {
    fn mark_paid(&mut self, id: String, amount: U128);
}

#[ext_contract(ext_ft)]
#[allow(dead_code)]
trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Vault {
    factory: AccountId,
    stablecoin: AccountId,
    invoice_id: String,
    amount_due: u128,
    paid: bool,
    withdrawn: bool,
}

#[near(serializers = [json])]
#[derive(Debug, Clone, PartialEq)]
pub struct VaultConfig {
    pub factory: AccountId,
    pub stablecoin: AccountId,
    pub invoice_id: String,
    pub amount_due: U128,
    pub paid: bool,
    pub withdrawn: bool,
}

#[near]
impl Vault {
    /// Called by the factor contract in the same batch that creates this account.
    #[init]
    pub fn new(factory: AccountId, stablecoin: AccountId, invoice_id: String, amount_due: U128) -> Self {
        require!(env::predecessor_account_id() == factory, "ERR_INIT_ONLY_FACTORY");
        require!(amount_due.0 > 0, "ERR_ZERO_AMOUNT");
        Self { factory, stablecoin, invoice_id, amount_due: amount_due.0, paid: false, withdrawn: false }
    }

    /// NEP-141 receiver. Returns the amount to refund to the payer.
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> PromiseOrValue<U128> {
        let _ = (sender_id, msg);
        require!(env::predecessor_account_id() == self.stablecoin, "ERR_WRONG_TOKEN");
        if self.paid || amount.0 < self.amount_due {
            return PromiseOrValue::Value(amount);
        }
        self.paid = true;
        ext_factor::ext(self.factory.clone())
            .with_static_gas(GAS_MARK_PAID)
            .mark_paid(self.invoice_id.clone(), U128(self.amount_due))
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_MARK_PAID).on_mark_paid(amount))
            .into()
    }

    #[private]
    pub fn on_mark_paid(&mut self, amount: U128, #[callback_result] r: Result<(), PromiseError>) -> U128 {
        if r.is_ok() {
            U128(amount.0 - self.amount_due)
        } else {
            self.paid = false;
            env::log_str("vault: factor rejected mark_paid, refunding payment");
            amount
        }
    }

    /// Only the factor contract may call this (after its owner + pending checks).
    /// Resolves to `true` iff the stablecoin transfer succeeded.
    pub fn withdraw_to(&mut self, to: AccountId) -> Promise {
        require!(env::predecessor_account_id() == self.factory, "ERR_ONLY_FACTORY");
        require!(self.paid, "ERR_NOT_PAID");
        require!(!self.withdrawn, "ERR_ALREADY_WITHDRAWN");
        self.withdrawn = true;
        ext_ft::ext(self.stablecoin.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FT_TRANSFER)
            .ft_transfer(to, U128(self.amount_due), Some(format!("FACTOR invoice #{}", self.invoice_id)))
            .then(Self::ext(env::current_account_id()).with_static_gas(GAS_ON_TRANSFERRED).on_transferred())
    }

    #[private]
    pub fn on_transferred(&mut self, #[callback_result] r: Result<(), PromiseError>) -> bool {
        if r.is_err() {
            self.withdrawn = false;
            env::log_str("vault: ft_transfer failed, funds stay in vault");
            return false;
        }
        true
    }

    pub fn get_config(&self) -> VaultConfig {
        VaultConfig {
            factory: self.factory.clone(),
            stablecoin: self.stablecoin.clone(),
            invoice_id: self.invoice_id.clone(),
            amount_due: U128(self.amount_due),
            paid: self.paid,
            withdrawn: self.withdrawn,
        }
    }

    pub fn is_paid(&self) -> bool {
        self.paid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{VMContextBuilder, accounts};
    use near_sdk::testing_env;

    const DUE: u128 = 500_000_000;

    fn factor() -> AccountId {
        "factor.testnet".parse().unwrap()
    }
    fn musdc() -> AccountId {
        "musdc.testnet".parse().unwrap()
    }
    fn as_caller(pred: AccountId) {
        testing_env!(
            VMContextBuilder::new()
                .current_account_id("inv-1.factor.testnet".parse().unwrap())
                .predecessor_account_id(pred)
                .account_balance(NearToken::from_millinear(10))
                .build()
        );
    }
    fn vault() -> Vault {
        as_caller(factor());
        Vault::new(factor(), musdc(), "1".into(), U128(DUE))
    }
    fn refund_of(r: PromiseOrValue<U128>) -> Option<u128> {
        match r {
            PromiseOrValue::Value(v) => Some(v.0),
            PromiseOrValue::Promise(_) => None,
        }
    }

    #[test]
    fn init_by_factory_and_view_config() {
        let v = vault();
        let c = v.get_config();
        assert_eq!(c.invoice_id, "1");
        assert_eq!(c.amount_due, U128(DUE));
        assert_eq!(c.stablecoin, musdc());
        assert!(!v.is_paid());
    }

    #[test]
    #[should_panic(expected = "ERR_INIT_ONLY_FACTORY")]
    fn init_by_stranger_fails() {
        as_caller(accounts(3));
        Vault::new(factor(), musdc(), "1".into(), U128(1));
    }

    #[test]
    #[should_panic(expected = "ERR_WRONG_TOKEN")]
    fn t1_fake_payment_rejected() {
        let mut v = vault();
        as_caller(accounts(3)); // attacker calls ft_on_transfer directly
        let _ = v.ft_on_transfer(accounts(3), U128(DUE), "".into());
    }

    #[test]
    fn t8_underpay_refunded_in_full() {
        let mut v = vault();
        as_caller(musdc());
        assert_eq!(refund_of(v.ft_on_transfer(accounts(1), U128(DUE - 1), "".into())), Some(DUE - 1));
        assert!(!v.is_paid());
    }

    #[test]
    fn exact_payment_marks_paid_and_calls_factor() {
        let mut v = vault();
        as_caller(musdc());
        assert_eq!(refund_of(v.ft_on_transfer(accounts(1), U128(DUE), "".into())), None);
        assert!(v.is_paid());
        // callback on success: refund 0
        assert_eq!(v.on_mark_paid(U128(DUE), Ok(())).0, 0);
    }

    #[test]
    fn excess_refunded_after_mark_paid() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE + 7), "".into());
        assert_eq!(v.on_mark_paid(U128(DUE + 7), Ok(())).0, 7);
        assert!(v.is_paid());
    }

    #[test]
    fn t9_double_pay_refunded_in_full() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE), "".into());
        v.on_mark_paid(U128(DUE), Ok(()));
        assert_eq!(refund_of(v.ft_on_transfer(accounts(1), U128(DUE), "".into())), Some(DUE));
    }

    #[test]
    fn t13_rejected_mark_paid_unwinds_and_refunds() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE + 3), "".into());
        let refund = v.on_mark_paid(U128(DUE + 3), Err(PromiseError::Failed));
        assert_eq!(refund.0, DUE + 3);
        assert!(!v.is_paid(), "payment must be retryable after factor rejected it");
    }

    #[test]
    #[should_panic(expected = "ERR_ONLY_FACTORY")]
    fn withdraw_to_only_factory() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE), "".into());
        v.on_mark_paid(U128(DUE), Ok(()));
        as_caller(accounts(2));
        let _ = v.withdraw_to(accounts(2));
    }

    #[test]
    #[should_panic(expected = "ERR_NOT_PAID")]
    fn withdraw_before_paid_fails() {
        let mut v = vault();
        let _ = v.withdraw_to(accounts(2));
    }

    #[test]
    fn withdraw_failure_allows_retry_success_blocks_second() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE), "".into());
        v.on_mark_paid(U128(DUE), Ok(()));
        as_caller(factor());
        let _ = v.withdraw_to(accounts(2));
        assert!(!v.on_transferred(Err(PromiseError::Failed)));
        assert!(!v.get_config().withdrawn);
        let _ = v.withdraw_to(accounts(2));
        assert!(v.on_transferred(Ok(())));
        assert!(v.get_config().withdrawn);
    }

    #[test]
    #[should_panic(expected = "ERR_ALREADY_WITHDRAWN")]
    fn withdraw_twice_fails() {
        let mut v = vault();
        as_caller(musdc());
        let _ = v.ft_on_transfer(accounts(1), U128(DUE), "".into());
        v.on_mark_paid(U128(DUE), Ok(()));
        as_caller(factor());
        let _ = v.withdraw_to(accounts(2));
        v.on_transferred(Ok(()));
        let _ = v.withdraw_to(accounts(2));
    }
}
