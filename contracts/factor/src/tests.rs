use super::*;
use near_contract_standards::non_fungible_token::core::{NonFungibleTokenCore, NonFungibleTokenResolver};
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::testing_env;

const DUE: u128 = 500_000_000;
const DAY: u64 = 86_400 * 1_000_000_000;
const T0: u64 = 1_800_000_000 * 1_000_000_000;

fn acc(s: &str) -> AccountId {
    s.parse().unwrap()
}
fn alice() -> AccountId {
    acc("alice.testnet")
}
fn carol() -> AccountId {
    acc("carol.testnet")
}
fn dao() -> AccountId {
    acc("marketing.sputnik-dao.testnet")
}
fn vault1() -> AccountId {
    acc("inv-1.factor.testnet")
}

fn ctx(pred: AccountId, deposit: NearToken, ts: u64) {
    testing_env!(
        VMContextBuilder::new()
            .current_account_id(acc("factor.testnet"))
            .predecessor_account_id(pred)
            .attached_deposit(deposit)
            .block_timestamp(ts)
            .account_balance(NearToken::from_near(100))
            .build()
    );
}
fn yocto(n: u128) -> NearToken {
    NearToken::from_yoctonear(n)
}

fn contract() -> Factor {
    ctx(acc("owner.testnet"), yocto(0), T0);
    Factor::new(
        acc("owner.testnet"),
        acc("musdc.testnet"),
        "6r3HfT3HcdGeoKKHmTyamoWdUufDmdNhCFukFXEvuMx".parse().unwrap(),
    )
}

/// Contract with invoice #1 by alice to dao, due in 30 days.
fn with_invoice() -> Factor {
    let mut c = contract();
    ctx(alice(), NearToken::from_millinear(100), T0);
    let id = c.create_invoice(dao(), U128(DUE), U64(T0 + 30 * DAY), "design work".into());
    assert_eq!(id, "1");
    c
}
fn accepted() -> Factor {
    let mut c = with_invoice();
    ctx(dao(), yocto(0), T0 + DAY);
    c.accept_invoice("1".into());
    c
}
fn listed(price: u128) -> Factor {
    let mut c = accepted();
    ctx(alice(), yocto(1), T0 + DAY);
    c.list("1".into(), U128(price));
    c
}
fn paid_at(mut c: Factor, ts: u64) -> Factor {
    ctx(vault1(), yocto(0), ts);
    c.mark_paid("1".into(), U128(DUE));
    c
}
fn status(c: &Factor) -> Status {
    c.invoice("1".into()).unwrap().status
}

// ------------------------------------------------------------------ create
#[test]
fn create_mints_nft_draft_with_vault_id() {
    let c = with_invoice();
    let v = c.invoice("1".into()).unwrap();
    assert_eq!(v.status, Status::Draft);
    assert_eq!(v.owner, alice());
    assert_eq!(v.vault, vault1());
    assert_eq!(v.amount, U128(DUE));
    assert_eq!(c.config().next_id, U64(2));
    assert_eq!(c.nft_token("1".into()).unwrap().owner_id, alice());
}

#[test]
#[should_panic(expected = "ERR_DEPOSIT")]
fn t11_create_requires_deposit() {
    let mut c = contract();
    ctx(alice(), NearToken::from_millinear(5), T0); // below VAULT_FUND + storage
    c.create_invoice(dao(), U128(DUE), U64(T0 + DAY), "".into());
}

#[test]
#[should_panic(expected = "ERR_DUE_IN_PAST")]
fn create_due_in_past_fails() {
    let mut c = contract();
    ctx(alice(), NearToken::from_millinear(100), T0);
    c.create_invoice(dao(), U128(DUE), U64(T0 - 1), "".into());
}

#[test]
#[should_panic(expected = "ERR_SELF_INVOICE")]
fn create_self_invoice_fails() {
    let mut c = contract();
    ctx(dao(), NearToken::from_millinear(100), T0);
    c.create_invoice(dao(), U128(DUE), U64(T0 + DAY), "".into());
}

#[test]
#[should_panic(expected = "ERR_MEMO_TOO_LONG")]
fn create_memo_limit() {
    let mut c = contract();
    ctx(alice(), NearToken::from_millinear(100), T0);
    c.create_invoice(dao(), U128(DUE), U64(T0 + DAY), "x".repeat(MAX_MEMO_LEN + 1));
}

// ------------------------------------------------------------------ accept
#[test]
fn accept_by_dao_updates_status_and_stats() {
    let c = accepted();
    assert_eq!(status(&c), Status::Accepted);
    let s = c.payer_stats(dao());
    assert_eq!(s.accepted, 1);
    assert_eq!(s.first_seen_ts, U64(T0 + DAY));
}

#[test]
#[should_panic(expected = "ERR_NOT_PAYER_DAO")]
fn t3_accept_only_dao() {
    let mut c = with_invoice();
    ctx(alice(), yocto(0), T0);
    c.accept_invoice("1".into());
}

#[test]
#[should_panic(expected = "ERR_BAD_STATUS")]
fn accept_twice_fails() {
    let mut c = accepted();
    ctx(dao(), yocto(0), T0 + DAY);
    c.accept_invoice("1".into());
}

// ------------------------------------------------------------------ list / buy
#[test]
#[should_panic(expected = "ERR_BAD_STATUS")]
fn list_draft_fails() {
    let mut c = with_invoice();
    ctx(alice(), yocto(1), T0);
    c.list("1".into(), U128(1));
}

#[test]
#[should_panic(expected = "ERR_NOT_OWNER")]
fn list_by_non_owner_fails() {
    let mut c = accepted();
    ctx(carol(), yocto(1), T0);
    c.list("1".into(), U128(1));
}

#[test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
fn list_requires_one_yocto() {
    let mut c = accepted();
    ctx(alice(), yocto(0), T0);
    c.list("1".into(), U128(1));
}

#[test]
fn list_and_delist() {
    let mut c = listed(9);
    let v = c.invoice("1".into()).unwrap();
    assert!(v.is_listed);
    assert_eq!(v.price, Some(U128(9)));
    assert_eq!(c.listings(None, None).len(), 1);
    ctx(alice(), yocto(1), T0);
    c.delist("1".into());
    assert!(!c.invoice("1".into()).unwrap().is_listed);
}

#[test]
fn buy_transfers_nft_and_clears_listing() {
    let price = NearToken::from_millinear(9_500).as_yoctonear();
    let mut c = listed(price);
    ctx(carol(), yocto(price + 123), T0 + 2 * DAY); // excess is refunded
    c.buy("1".into());
    let v = c.invoice("1".into()).unwrap();
    assert_eq!(v.owner, carol());
    assert!(!v.is_listed);
    assert!(c.listings(None, None).is_empty());
    assert_eq!(c.invoices_by_owner(carol()).len(), 1);
    assert!(c.invoices_by_owner(alice()).is_empty());
}

#[test]
#[should_panic(expected = "ERR_PRICE_TOO_LOW")]
fn buy_below_price_fails() {
    let mut c = listed(1_000);
    ctx(carol(), yocto(999), T0);
    c.buy("1".into());
}

#[test]
#[should_panic(expected = "ERR_OWN_LISTING")]
fn buy_own_listing_fails() {
    let mut c = listed(1_000);
    ctx(alice(), yocto(1_000), T0);
    c.buy("1".into());
}

#[test]
#[should_panic(expected = "ERR_NOT_LISTED")]
fn buy_unlisted_fails() {
    let mut c = accepted();
    ctx(carol(), yocto(1_000), T0);
    c.buy("1".into());
}

// ------------------------------------------------------------------ paid
#[test]
fn mark_paid_on_time_updates_stats_and_clears_listing() {
    let c = paid_at(listed(1_000), T0 + 10 * DAY);
    let v = c.invoice("1".into()).unwrap();
    assert_eq!(v.status, Status::Paid);
    assert_eq!(v.paid_on_time, Some(true));
    assert!(!v.is_listed, "T12: payment clears listing");
    let s = c.payer_stats(dao());
    assert_eq!((s.paid_on_time, s.paid_late, s.volume_paid), (1, 0, U128(DUE)));
}

#[test]
fn late_payment_counts_late() {
    let c = paid_at(accepted(), T0 + 31 * DAY);
    let s = c.payer_stats(dao());
    assert_eq!((s.paid_on_time, s.paid_late), (0, 1));
    assert_eq!(c.invoice("1".into()).unwrap().paid_on_time, Some(false));
}

#[test]
fn overdue_is_derived() {
    let c = accepted();
    ctx(carol(), yocto(0), T0 + 31 * DAY);
    assert!(c.invoice("1".into()).unwrap().is_overdue);
    ctx(carol(), yocto(0), T0 + 29 * DAY);
    assert!(!c.invoice("1".into()).unwrap().is_overdue);
}

#[test]
#[should_panic(expected = "ERR_ONLY_VAULT")]
fn t2_mark_paid_only_vault() {
    let mut c = accepted();
    ctx(carol(), yocto(0), T0);
    c.mark_paid("1".into(), U128(DUE));
}

#[test]
#[should_panic(expected = "ERR_BAD_STATUS")]
fn mark_paid_on_draft_fails_so_vault_refunds() {
    let mut c = with_invoice();
    ctx(vault1(), yocto(0), T0);
    c.mark_paid("1".into(), U128(DUE));
}

#[test]
#[should_panic(expected = "ERR_BAD_STATUS")]
fn mark_paid_twice_fails() {
    let mut c = paid_at(accepted(), T0 + DAY);
    ctx(vault1(), yocto(0), T0 + DAY);
    c.mark_paid("1".into(), U128(DUE));
}

// ------------------------------------------------------------------ withdraw
fn sold_and_paid() -> Factor {
    let mut c = listed(1_000);
    ctx(carol(), yocto(1_000), T0 + 2 * DAY);
    c.buy("1".into());
    paid_at(c, T0 + 3 * DAY)
}

#[test]
#[should_panic(expected = "ERR_NOT_OWNER")]
fn t4_old_owner_withdraw_fails() {
    let mut c = sold_and_paid();
    ctx(alice(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
}

#[test]
#[should_panic(expected = "ERR_NOT_PAID")]
fn withdraw_before_paid_fails() {
    let mut c = accepted();
    ctx(alice(), yocto(1), T0);
    let _ = c.withdraw("1".into());
}

#[test]
fn withdraw_sets_pending_then_success_settles() {
    let mut c = sold_and_paid();
    ctx(carol(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
    assert!(c.invoice("1".into()).unwrap().pending);
    ctx(acc("factor.testnet"), yocto(0), T0 + 4 * DAY);
    assert!(c.on_withdraw("1".into(), carol(), Ok(true)));
    let v = c.invoice("1".into()).unwrap();
    assert_eq!(v.status, Status::Settled);
    assert!(!v.pending);
}

#[test]
fn t6_withdraw_failure_unlocks_and_stays_paid() {
    let mut c = sold_and_paid();
    ctx(carol(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
    ctx(acc("factor.testnet"), yocto(0), T0 + 4 * DAY);
    assert!(!c.on_withdraw("1".into(), carol(), Err(PromiseError::Failed)));
    let v = c.invoice("1".into()).unwrap();
    assert_eq!((v.status, v.pending), (Status::Paid, false));
    // vault answered `false` (its transfer failed) -> also failure
    ctx(carol(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
    ctx(acc("factor.testnet"), yocto(0), T0 + 4 * DAY);
    assert!(!c.on_withdraw("1".into(), carol(), Ok(false)));
    assert_eq!(status(&c), Status::Paid);
}

#[test]
#[should_panic(expected = "ERR_PENDING")]
fn t5_transfer_blocked_while_pending() {
    let mut c = sold_and_paid();
    ctx(carol(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
    c.nft_transfer(alice(), "1".into(), None, None);
}

#[test]
#[should_panic(expected = "ERR_PENDING")]
fn t5_withdraw_twice_while_pending() {
    let mut c = sold_and_paid();
    ctx(carol(), yocto(1), T0 + 4 * DAY);
    let _ = c.withdraw("1".into());
    let _ = c.withdraw("1".into());
}

#[test]
#[should_panic(expected = "ERR_PENDING")]
fn pending_blocks_list_and_buy() {
    // a pending nft_transfer_call locks list/buy too
    let mut c = listed(1_000);
    ctx(alice(), yocto(1), T0);
    let _ = c.nft_transfer_call(acc("market.testnet"), "1".into(), None, None, "".into());
    ctx(acc("market.testnet"), yocto(1), T0); // temporary owner can't list while in flight
    c.list("1".into(), U128(5));
}

#[test]
fn nft_transfer_call_lock_released_by_resolver() {
    let mut c = accepted();
    ctx(alice(), yocto(1), T0);
    let _ = c.nft_transfer_call(acc("market.testnet"), "1".into(), None, None, "".into());
    assert!(c.invoice("1".into()).unwrap().pending);
    // receiver's nft_on_transfer returned `false` (keep the token)
    testing_env!(
        VMContextBuilder::new()
            .current_account_id(acc("factor.testnet"))
            .predecessor_account_id(acc("factor.testnet"))
            .block_timestamp(T0)
            .build(),
        near_sdk::test_vm_config(),
        near_sdk::RuntimeFeesConfig::test(),
        Default::default(),
        vec![near_sdk::PromiseResult::Successful(b"false".to_vec())],
    );
    c.nft_resolve_transfer(alice(), acc("market.testnet"), "1".into(), None);
    let v = c.invoice("1".into()).unwrap();
    assert!(!v.pending);
    assert_eq!(v.owner, acc("market.testnet"));
}

#[test]
fn t12_listing_cleared_on_transfer() {
    let mut c = listed(1_000);
    ctx(alice(), yocto(1), T0);
    c.nft_transfer(carol(), "1".into(), None, None);
    let v = c.invoice("1".into()).unwrap();
    assert_eq!(v.owner, carol());
    assert!(!v.is_listed);
}

// ------------------------------------------------------------------ metadata
#[test]
fn nft_metadata_is_live() {
    let c = accepted();
    let t = c.nft_token("1".into()).unwrap();
    let m = t.metadata.unwrap();
    assert_eq!(m.title.as_deref(), Some("FACTOR Invoice #1"));
    let d = m.description.unwrap();
    assert!(d.contains("500.00 mUSDC owed by marketing.sputnik-dao.testnet"), "{d}");
    assert!(d.contains("status ACCEPTED"), "{d}");
    assert!(d.contains("payer score 0/100"), "{d}");
    let extra: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&m.extra.unwrap()).unwrap();
    assert_eq!(extra["status"], "Accepted");
    assert_eq!(extra["vault"], "inv-1.factor.testnet");
}

#[test]
fn date_and_amount_format() {
    assert_eq!(views::fmt_date(0), "1970-01-01");
    assert_eq!(views::fmt_date(1_794_614_400 * 1_000_000_000), "2026-11-14");
    assert_eq!(views::fmt_amount(500_000_000), "500.00");
    assert_eq!(views::fmt_amount(1_234_567), "1.23");
}
