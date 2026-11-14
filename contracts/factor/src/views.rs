//! Read-only views. Derived values (overdue, listed, payer score) are computed here (PRD §12).
use crate::score::{NS_PER_DAY, payer_score};
use crate::{Factor, FactorExt, Invoice, Status};
use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::json_types::{Base58CryptoHash, U64, U128};
use near_sdk::{AccountId, env, near};

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct InvoiceView {
    pub id: TokenId,
    pub owner: AccountId,
    pub creator: AccountId,
    pub payer_dao: AccountId,
    pub vault: AccountId,
    pub amount: U128,
    pub due_ts: U64,
    pub created_ts: U64,
    pub accepted_ts: Option<U64>,
    pub paid_ts: Option<U64>,
    pub settled_ts: Option<U64>,
    pub paid_on_time: Option<bool>,
    pub status: Status,
    pub pending: bool,
    pub memo: String,
    pub is_overdue: bool,
    pub is_listed: bool,
    /// Listing price in yoctoNEAR, if listed.
    pub price: Option<U128>,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct PayerStatsView {
    pub dao: AccountId,
    pub accepted: u32,
    pub paid_on_time: u32,
    pub paid_late: u32,
    pub volume_paid: U128,
    pub first_seen_ts: U64,
    pub age_days: u64,
    pub on_time_rate_pct: u32,
    pub payer_score: u32,
}

#[near(serializers = [json])]
pub struct ConfigView {
    pub owner_id: AccountId,
    pub stablecoin: AccountId,
    pub vault_code_hash: Base58CryptoHash,
    pub next_id: U64,
    pub create_fixed_cost: U128,
}

impl Factor {
    pub(crate) fn invoice_view(&self, inv: &Invoice, owner: AccountId) -> InvoiceView {
        let listing = self.listings.get(&inv.id);
        InvoiceView {
            id: inv.id.clone(),
            owner,
            creator: inv.creator.clone(),
            payer_dao: inv.payer_dao.clone(),
            vault: inv.vault.clone(),
            amount: U128(inv.amount),
            due_ts: U64(inv.due_ts),
            created_ts: U64(inv.created_ts),
            accepted_ts: inv.accepted_ts.map(U64),
            paid_ts: inv.paid_ts.map(U64),
            settled_ts: inv.settled_ts.map(U64),
            paid_on_time: inv.paid_on_time,
            status: inv.status,
            pending: inv.pending,
            memo: inv.memo.clone(),
            is_overdue: inv.status == Status::Accepted && env::block_timestamp() > inv.due_ts,
            is_listed: listing.is_some(),
            price: listing.map(|l| U128(l.price)),
        }
    }
}

#[near]
impl Factor {
    pub fn invoice(&self, id: TokenId) -> Option<InvoiceView> {
        let inv = self.invoices.get(&id)?;
        Some(self.invoice_view(inv, self.owner_of(&id)))
    }

    pub fn payer_stats(&self, dao: AccountId) -> PayerStatsView {
        let s = self.payer_stats.get(&dao).cloned().unwrap_or_default();
        let age_days = if s.first_seen_ts == 0 {
            0
        } else {
            env::block_timestamp().saturating_sub(s.first_seen_ts) / NS_PER_DAY
        };
        let (score, rate) = payer_score(s.paid_on_time, s.paid_late, s.volume_paid, age_days);
        PayerStatsView {
            dao,
            accepted: s.accepted,
            paid_on_time: s.paid_on_time,
            paid_late: s.paid_late,
            volume_paid: U128(s.volume_paid),
            first_seen_ts: U64(s.first_seen_ts),
            age_days,
            on_time_rate_pct: rate,
            payer_score: score,
        }
    }

    pub fn listings(&self, from_index: Option<u32>, limit: Option<u32>) -> Vec<InvoiceView> {
        self.listings
            .keys()
            .skip(from_index.unwrap_or(0) as usize)
            .take(limit.unwrap_or(50).min(100) as usize)
            .filter_map(|id| self.invoice(id.clone()))
            .collect()
    }

    pub fn invoices_by_owner(&self, owner: AccountId) -> Vec<InvoiceView> {
        let Some(per_owner) = self.tokens.tokens_per_owner.as_ref() else { return vec![] };
        let Some(ids) = per_owner.get(&owner) else { return vec![] };
        ids.iter().filter_map(|id| self.invoice(id)).collect()
    }

    /// Most recent invoices first (for the UI); `limit` max 100.
    pub fn recent_invoices(&self, limit: Option<u32>) -> Vec<InvoiceView> {
        let limit = limit.unwrap_or(30).min(100) as u64;
        let last = self.next_id.saturating_sub(1);
        (last.saturating_sub(limit.saturating_sub(1)).max(1)..=last)
            .rev()
            .filter_map(|n| self.invoice(n.to_string()))
            .collect()
    }

    pub fn config(&self) -> ConfigView {
        ConfigView {
            owner_id: self.owner_id.clone(),
            stablecoin: self.stablecoin.clone(),
            vault_code_hash: self.vault_code_hash,
            next_id: U64(self.next_id),
            create_fixed_cost: U128(
                crate::VAULT_FUND.saturating_add(crate::STABLECOIN_STORAGE_DEPOSIT).as_yoctonear(),
            ),
        }
    }
}

/// 500000000 (6 decimals) -> "500.00"
pub fn fmt_amount(base: u128) -> String {
    let whole = base / 1_000_000;
    let cents = (base % 1_000_000) / 10_000;
    format!("{whole}.{cents:02}")
}

/// ns timestamp -> "YYYY-MM-DD" (UTC), civil-from-days (H. Hinnant).
pub fn fmt_date(ts_ns: u64) -> String {
    let z = (ts_ns / NS_PER_DAY) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}
