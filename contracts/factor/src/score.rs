//! Payer score (PRD §12), 0–100:
//!   60 × on_time_rate + 25 × min(1, log10(volume_whole_mUSDC) / 6) + 15 × min(1, age_days / 180)
//! Volume- and age-weighted so tiny fake histories score low (T10).
//! `age_days` is measured from the DAO's first acceptance on FACTOR (`first_seen_ts`): a contract
//! cannot read an account's creation time (see DECISIONS.md).

pub const STABLE_DECIMALS: u32 = 6;
pub const NS_PER_DAY: u64 = 86_400 * 1_000_000_000;

/// Returns (score 0..=100, on_time_rate in percent).
pub fn payer_score(paid_on_time: u32, paid_late: u32, volume_paid: u128, age_days: u64) -> (u32, u32) {
    let paid = paid_on_time as f64 + paid_late as f64;
    let on_time_rate = if paid > 0.0 { paid_on_time as f64 / paid } else { 0.0 };
    let whole = volume_paid as f64 / 10f64.powi(STABLE_DECIMALS as i32);
    let vol = if whole > 1.0 { (whole.log10() / 6.0).min(1.0) } else { 0.0 };
    let age = (age_days as f64 / 180.0).min(1.0);
    let score = 60.0 * on_time_rate + 25.0 * vol + 15.0 * age;
    (score.round().clamp(0.0, 100.0) as u32, (on_time_rate * 100.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    const M: u128 = 1_000_000; // 1 mUSDC

    #[test]
    fn zero_history_scores_zero() {
        assert_eq!(payer_score(0, 0, 0, 0), (0, 0));
    }

    #[test]
    fn perfect_record_scores_100() {
        assert_eq!(payer_score(10, 0, 1_000_000 * M, 365).0, 100);
    }

    #[test]
    fn tiny_sybil_history_scores_low() {
        // fake DAO paying itself one 1-mUSDC invoice on time, brand-new
        let (s, rate) = payer_score(1, 0, M, 0);
        assert_eq!(rate, 100);
        assert_eq!(s, 60, "volume and age add nothing");
        assert!(s < 75, "must not reach the green badge");
    }

    #[test]
    fn demo_dao_one_on_time_500() {
        // 60 + 25*log10(500)/6 (=11.2) + 0
        assert_eq!(payer_score(1, 0, 500 * M, 0).0, 71);
    }

    #[test]
    fn late_payments_hurt() {
        let (s, rate) = payer_score(1, 1, 1_000 * M, 30);
        assert_eq!(rate, 50);
        assert_eq!(s, 45); // 30 + 12.5 + 2.5
    }

    #[test]
    fn caps_hold() {
        assert_eq!(payer_score(1, 0, u128::MAX, u64::MAX).0, 100);
    }
}
