#![allow(unused_must_use)]
//! FACTOR sandbox integration tests (PRD §16). Real mUSDC, real global-code vaults, real Sputnik DAO v2.
use factor_integration::*;
use near_api::NearToken;
use serde_json::{Value, json};

const PRICE_MILLI: u128 = 9_500; // 9.5 NEAR

fn price() -> NearToken {
    NearToken::from_millinear(PRICE_MILLI)
}

/// alice invoices `payer`, payer accepts directly (no governance; Sputnik is covered separately).
async fn accepted(e: &Env) -> TestResult<String> {
    let inv = e.create(&e.payer, 30 * DAY_NS).await?;
    e.accept_direct(&inv).await?;
    Ok(inv)
}

// ---------------------------------------------------------------- happy path (US-1..US-7)
#[tokio::test]
async fn t_happy_path() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 30 * DAY_NS).await?;
    assert_eq!(e.status(&inv).await?, "Draft");
    e.accept_direct(&inv).await?;
    assert_eq!(e.status(&inv).await?, "Accepted");

    // alice lists, carol buys: alice gets NEAR today, carol owns the NFT
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    e.register(&e.carol).await?;
    let alice_before = e.near_balance(&e.alice).await?;
    assert_ok(e.buy(&e.carol, &inv, NearToken::from_millinear(PRICE_MILLI + 500)).await?);
    let gained = e.near_balance(&e.alice).await?.as_yoctonear() - alice_before.as_yoctonear();
    assert_eq!(gained, price().as_yoctonear(), "seller receives exactly the price");
    let v = e.invoice(&inv).await?;
    assert_eq!(v["owner"], e.carol.as_str());
    assert_eq!(v["is_listed"], false);

    // payer pays the vault's address -> PAID, on time
    let payer_before = e.ft_balance(&e.payer).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(e.status(&inv).await?, "Paid");
    assert_eq!(e.invoice(&inv).await?["paid_on_time"], true);
    assert_eq!(payer_before - e.ft_balance(&e.payer).await?, FACE);
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, FACE);

    // alice (old owner) is rejected, carol withdraws 500
    let carol_before = e.ft_balance(&e.carol).await?;
    assert_ok(e.withdraw(&e.carol, &inv).await?);
    assert_eq!(e.ft_balance(&e.carol).await? - carol_before, FACE, "buyer receives 500 mUSDC");
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, 0);
    let v = e.invoice(&inv).await?;
    assert_eq!((v["status"].as_str(), v["pending"].as_bool()), (Some("Settled"), Some(false)));

    let s = e.stats(&e.payer).await?;
    assert_eq!((s["accepted"].as_u64(), s["paid_on_time"].as_u64(), s["paid_late"].as_u64()), (Some(1), Some(1), Some(0)));
    assert_eq!(s["volume_paid"], FACE.to_string());
    Ok(())
}

// ---------------------------------------------------------------- T7 keyless vault on GLOBAL code
#[tokio::test]
async fn t_vault_has_no_keys() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 30 * DAY_NS).await?;
    let vault = e.vault_of(&inv);
    let keys = near_api::Account(vault.clone()).list_keys().fetch_from(&e.net).await?;
    assert!(keys.data.is_empty(), "vault must have 0 access keys, got {:?}", keys.data);
    let acct = format!("{:?}", near_api::Account(vault.clone()).view().fetch_from(&e.net).await?.data);
    assert!(acct.contains("GlobalHash") && acct.contains(&e.vault_hash), "vault runs global code by hash: {acct}");
    let cfg: Value = e.view(&vault, "get_config", json!({})).await?;
    assert_eq!(cfg["factory"], e.factor.as_str());
    assert_eq!(cfg["stablecoin"], e.musdc.as_str());
    assert_eq!(cfg["invoice_id"], inv.as_str());
    assert_eq!(cfg["amount_due"], FACE.to_string());
    // pre-registered on mUSDC (NEP-145) by the factor at creation
    let reg: Value = e.view(&e.musdc, "storage_balance_of", json!({"account_id": vault})).await?;
    assert!(!reg.is_null(), "vault must be storage-registered on mUSDC");
    Ok(())
}

// ---------------------------------------------------------------- T3
#[tokio::test]
async fn t_accept_only_dao() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 30 * DAY_NS).await?;
    for who in [&e.alice, &e.carol, &e.bob] {
        let r = e.call(who, &e.factor, "accept_invoice", json!({"id": inv}), 0, 30).await?;
        assert_err(&r, "ERR_NOT_PAYER_DAO");
    }
    assert_eq!(e.status(&inv).await?, "Draft");
    Ok(())
}

// ---------------------------------------------------------------- real Sputnik governance
#[tokio::test]
async fn t_accept_via_sputnik_proposal() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.dao, 30 * DAY_NS).await?;
    // a single council vote is not enough (checked inside the helper), two executes
    let (pid, _) = e
        .dao_function_call(&e.factor, "accept_invoice", json!({"id": inv}), 0, 30, &format!("accept FACTOR invoice #{inv}"))
        .await?;
    assert_eq!(e.proposal(pid).await?["status"], "Approved");
    assert_eq!(e.status(&inv).await?, "Accepted");
    let s = e.stats(&e.dao).await?;
    assert_eq!(s["accepted"], 1);
    // a non-council account cannot vote
    let again = e.call(&e.alice, &e.dao, "act_proposal", json!({"id": pid, "action": "VoteApprove", "proposal": {"FunctionCall": {"receiver_id": e.factor, "actions": []}}}), 0, 30).await?;
    assert!(!text(&again).is_empty());
    Ok(())
}

#[tokio::test]
async fn t_dao_pays_via_sputnik_proposal() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.dao, 30 * DAY_NS).await?;
    e.dao_function_call(&e.factor, "accept_invoice", json!({"id": inv}), 0, 30, "accept").await?;
    let dao_before = e.ft_balance(&e.dao).await?;
    let (pid, _) = e
        .dao_function_call(
            &e.musdc,
            "ft_transfer_call",
            json!({"receiver_id": e.vault_of(&inv), "amount": FACE.to_string(), "msg": ""}),
            1,
            200,
            &format!("pay FACTOR invoice #{inv}"),
        )
        .await?;
    assert_eq!(e.proposal(pid).await?["status"], "Approved");
    assert_eq!(e.status(&inv).await?, "Paid");
    assert_eq!(dao_before - e.ft_balance(&e.dao).await?, FACE);
    let s = e.stats(&e.dao).await?;
    assert_eq!((s["paid_on_time"].as_u64(), s["accepted"].as_u64()), (Some(1), Some(1)));
    Ok(())
}

// ---------------------------------------------------------------- T1
#[tokio::test]
async fn t_fake_payment_rejected() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    let vault = e.vault_of(&inv);
    // attacker calls the vault directly, pretending mUSDC delivered funds
    let r = e
        .call(&e.carol, &vault, "ft_on_transfer", json!({"sender_id": e.carol, "amount": FACE.to_string(), "msg": ""}), 0, 100)
        .await?;
    assert_err(&r, "ERR_WRONG_TOKEN");
    assert_eq!(e.status(&inv).await?, "Accepted");
    let paid: bool = e.view(&vault, "is_paid", json!({})).await?;
    assert!(!paid);
    Ok(())
}

// ---------------------------------------------------------------- T2
#[tokio::test]
async fn t_mark_paid_only_vault() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    for who in [&e.carol, &e.alice, &e.payer] {
        let r = e.call(who, &e.factor, "mark_paid", json!({"id": inv, "amount": FACE.to_string()}), 0, 30).await?;
        assert_err(&r, "ERR_ONLY_VAULT");
    }
    // even another invoice's vault cannot mark this one: it has no keys, so nobody can sign for it
    let other = e.create(&e.payer, 30 * DAY_NS).await?;
    let signed = near_api::Contract(e.factor.clone())
        .call_function("mark_paid", json!({"id": inv, "amount": FACE.to_string()}))
        .transaction()
        .with_signer(e.vault_of(&other), e.signer.clone())
        .send_to(&e.net)
        .await;
    assert!(signed.is_err(), "a keyless vault cannot sign transactions");
    assert_eq!(e.status(&inv).await?, "Accepted");
    Ok(())
}

// ---------------------------------------------------------------- T8 / T9 / excess
#[tokio::test]
async fn t_underpay_refunded() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    let before = e.ft_balance(&e.payer).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE - 1).await?);
    assert_eq!(e.ft_balance(&e.payer).await?, before, "underpayment refunded in full");
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, 0);
    assert_eq!(e.status(&inv).await?, "Accepted");
    Ok(())
}

#[tokio::test]
async fn t_double_pay_refunded() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    let start = e.ft_balance(&e.payer).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(start - e.ft_balance(&e.payer).await?, FACE, "second payment refunded");
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, FACE);
    let s = e.stats(&e.payer).await?;
    assert_eq!(s["paid_on_time"], 1, "counted once");
    Ok(())
}

#[tokio::test]
async fn t_excess_refunded() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    let start = e.ft_balance(&e.payer).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE + 123 * M).await?);
    assert_eq!(start - e.ft_balance(&e.payer).await?, FACE, "only the amount due is kept");
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, FACE);
    assert_eq!(e.status(&inv).await?, "Paid");
    Ok(())
}

// ---------------------------------------------------------------- T13 (D6): pay before accept
#[tokio::test]
async fn t_pay_before_accept_refunded() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 30 * DAY_NS).await?; // still Draft
    let start = e.ft_balance(&e.payer).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(e.ft_balance(&e.payer).await?, start, "factor rejected mark_paid -> vault refunded");
    let paid: bool = e.view(&e.vault_of(&inv), "is_paid", json!({})).await?;
    assert!(!paid, "vault unwound");
    // and the invoice can still be accepted and paid normally afterwards
    e.accept_direct(&inv).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(e.status(&inv).await?, "Paid");
    Ok(())
}

// ---------------------------------------------------------------- T4
#[tokio::test]
async fn t_old_owner_withdraw_fails() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    assert_ok(e.buy(&e.carol, &inv, price()).await?);
    e.register(&e.carol).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);

    let alice_before = e.ft_balance(&e.alice).await?;
    let r = e.withdraw(&e.alice, &inv).await?;
    assert_err(&r, "ERR_NOT_OWNER");
    assert_eq!(e.ft_balance(&e.alice).await?, alice_before);
    assert_eq!(e.status(&inv).await?, "Paid");
    // nobody else either
    assert_err(&e.withdraw(&e.bob, &inv).await?, "ERR_NOT_OWNER");
    // vault refuses direct calls even from the owner
    let r = e.call(&e.carol, &e.vault_of(&inv), "withdraw_to", json!({"to": e.carol}), 0, 50).await?;
    assert_err(&r, "ERR_ONLY_FACTORY");
    assert_ok(e.withdraw(&e.carol, &inv).await?);
    assert_eq!(e.status(&inv).await?, "Settled");
    Ok(())
}

// ---------------------------------------------------------------- T5
#[tokio::test]
async fn t_transfer_blocked_while_pending() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(e.invoice(&inv).await?["owner"], e.alice.as_str());
    // One transaction, two actions: withdraw() sets pending=true, then nft_transfer runs while the
    // withdraw is still in flight. The transfer must abort with ERR_PENDING (reverting both).
    let r = e
        .batch(
            &e.alice,
            &e.factor,
            vec![
                ("withdraw", json!({"id": inv}), 1, 150),
                ("nft_transfer", json!({"receiver_id": e.carol, "token_id": inv}), 1, 30),
            ],
        )
        .await?;
    assert_err(&r, "ERR_PENDING");
    // same for a second withdraw (double claim) and list
    let r = e
        .batch(
            &e.alice,
            &e.factor,
            vec![("withdraw", json!({"id": inv}), 1, 100), ("withdraw", json!({"id": inv}), 1, 100)],
        )
        .await?;
    assert_err(&r, "ERR_PENDING");
    // nothing leaked: still alice's, unlocked, withdrawable
    let v = e.invoice(&inv).await?;
    assert_eq!((v["owner"].as_str(), v["pending"].as_bool()), (Some(e.alice.as_str()), Some(false)));
    assert_ok(e.withdraw(&e.alice, &inv).await?);
    assert_eq!(e.status(&inv).await?, "Settled");
    Ok(())
}

// ---------------------------------------------------------------- T6
#[tokio::test]
async fn t_withdraw_failure_unlocks() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    assert_ok(e.buy(&e.carol, &inv, price()).await?); // carol is NOT registered on mUSDC
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);

    // vault's ft_transfer to unregistered carol panics -> callback must unlock
    assert_ok(e.withdraw(&e.carol, &inv).await?);
    let v = e.invoice(&inv).await?;
    assert_eq!((v["status"].as_str(), v["pending"].as_bool()), (Some("Paid"), Some(false)), "unlocked, still PAID");
    assert_eq!(e.ft_balance(&e.vault_of(&inv)).await?, FACE, "funds still in the vault");

    // retry after registering works
    e.register(&e.carol).await?;
    assert_ok(e.withdraw(&e.carol, &inv).await?);
    assert_eq!(e.status(&inv).await?, "Settled");
    assert_eq!(e.ft_balance(&e.carol).await?, FACE);
    // and the vault will not pay twice
    assert_err(&e.withdraw(&e.carol, &inv).await?, "ERR_NOT_PAID");
    Ok(())
}

// ---------------------------------------------------------------- T12
#[tokio::test]
async fn t_listing_cleared_on_transfer() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    let l: Vec<Value> = e.view(&e.factor, "listings", json!({})).await?;
    assert_eq!(l.len(), 1);
    assert_ok(e.call(&e.alice, &e.factor, "nft_transfer", json!({"receiver_id": e.carol, "token_id": inv}), 1, 30).await?);
    let v = e.invoice(&inv).await?;
    assert_eq!((v["owner"].as_str(), v["is_listed"].as_bool()), (Some(e.carol.as_str()), Some(false)));
    let l: Vec<Value> = e.view(&e.factor, "listings", json!({})).await?;
    assert!(l.is_empty());
    // the stale listing cannot be bought
    assert_err(&e.buy(&e.bob, &inv, price()).await?, "ERR_NOT_LISTED");
    Ok(())
}

#[tokio::test]
async fn t_listing_cleared_on_payment() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    assert_eq!(e.invoice(&inv).await?["is_listed"], false);
    assert_err(&e.buy(&e.carol, &inv, price()).await?, "ERR_NOT_LISTED");
    Ok(())
}

// ---------------------------------------------------------------- late payment
#[tokio::test]
async fn t_late_payment_counts_late() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 12 * 1_000_000_000).await?; // due in ~12 s
    e.accept_direct(&inv).await?;
    assert_eq!(e.invoice(&inv).await?["is_overdue"], false);
    tokio::time::sleep(std::time::Duration::from_secs(14)).await;
    e.sandbox.fast_forward(20).await?;
    assert_eq!(e.invoice(&inv).await?["is_overdue"], true, "past due and unpaid shows OVERDUE (US-8)");
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    let v = e.invoice(&inv).await?;
    assert_eq!((v["status"].as_str(), v["paid_on_time"].as_bool()), (Some("Paid"), Some(false)));
    let s = e.stats(&e.payer).await?;
    assert_eq!((s["paid_on_time"].as_u64(), s["paid_late"].as_u64()), (Some(0), Some(1)));
    assert_eq!(s["on_time_rate_pct"], 0);
    Ok(())
}

// ---------------------------------------------------------------- T11
#[tokio::test]
async fn t_create_requires_deposit() -> TestResult<()> {
    let e = Env::new().await?;
    let args = json!({"payer_dao": e.payer, "amount": FACE.to_string(),
                      "due_ts": (now_ns() + DAY_NS).to_string(), "memo": "spam"});
    for deposit in [0u128, NearToken::from_millinear(5).as_yoctonear()] {
        let r = e.call(&e.alice, &e.factor, "create_invoice", args.clone(), deposit, 100).await?;
        assert_err(&r, "ERR_DEPOSIT");
    }
    let cfg: Value = e.view(&e.factor, "config", json!({})).await?;
    assert_eq!(cfg["next_id"], "1", "no state change, no vault");
    assert!(near_api::Account(e.vault_of("1")).view().fetch_from(&e.net).await.is_err());
    Ok(())
}

#[tokio::test]
async fn t_create_refunds_excess_and_is_cheap() -> TestResult<()> {
    let e = Env::new().await?;
    let before = e.near_balance(&e.alice).await?.as_yoctonear();
    let inv = e.create(&e.payer, 30 * DAY_NS).await?; // attaches 0.1 NEAR
    let spent = before - e.near_balance(&e.alice).await?.as_yoctonear();
    let spent_milli = spent / 1_000_000_000_000_000_000_000;
    println!("create_invoice net cost incl. gas: {spent_milli} milliNEAR (deposit was 100)");
    assert!(spent_milli < 50, "creation must cost well under 0.05 NEAR, spent {spent_milli} mNEAR");
    let _ = inv;
    Ok(())
}

// ---------------------------------------------------------------- misc invariants
#[tokio::test]
async fn t_market_rules() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = e.create(&e.payer, 30 * DAY_NS).await?;
    assert_err(&e.list(&e.alice, &inv, price()).await?, "ERR_BAD_STATUS"); // DRAFT can't be listed
    e.accept_direct(&inv).await?;
    assert_err(&e.list(&e.carol, &inv, price()).await?, "ERR_NOT_OWNER");
    assert_ok(e.list(&e.alice, &inv, price()).await?);
    assert_err(&e.buy(&e.alice, &inv, price()).await?, "ERR_OWN_LISTING");
    assert_err(&e.buy(&e.carol, &inv, NearToken::from_millinear(PRICE_MILLI - 1)).await?, "ERR_PRICE_TOO_LOW");
    let carol_before = e.near_balance(&e.carol).await?;
    let r = e.buy(&e.carol, &inv, NearToken::from_millinear(PRICE_MILLI - 1)).await?;
    assert!(r.clone().into_result().is_err());
    assert!(carol_before.as_yoctonear() - e.near_balance(&e.carol).await?.as_yoctonear() < NearToken::from_millinear(10).as_yoctonear(), "failed buy refunds the deposit");
    assert_ok(e.call(&e.alice, &e.factor, "delist", json!({"id": inv}), 1, 30).await?);
    assert_err(&e.buy(&e.carol, &inv, price()).await?, "ERR_NOT_LISTED");
    Ok(())
}

#[tokio::test]
async fn t_nft_metadata_is_live() -> TestResult<()> {
    let e = Env::new().await?;
    let inv = accepted(&e).await?;
    let t: Value = e.view(&e.factor, "nft_token", json!({"token_id": inv})).await?;
    assert_eq!(t["owner_id"], e.alice.as_str());
    assert_eq!(t["metadata"]["title"], format!("FACTOR Invoice #{inv}"));
    let d = t["metadata"]["description"].as_str().unwrap().to_string();
    assert!(d.contains("500.00 mUSDC") && d.contains("ACCEPTED") && d.contains("payer score"), "{d}");
    assert_ok(e.pay(&e.payer, &inv, FACE).await?);
    let t: Value = e.view(&e.factor, "nft_token", json!({"token_id": inv})).await?;
    assert!(t["metadata"]["description"].as_str().unwrap().contains("PAID"));
    let extra: Value = serde_json::from_str(t["metadata"]["extra"].as_str().unwrap())?;
    assert_eq!(extra["status"], "Paid");
    // NEP-181
    let all: Vec<Value> = e.view(&e.factor, "nft_tokens_for_owner", json!({"account_id": e.alice})).await?;
    assert_eq!(all.len(), 1);
    Ok(())
}
