use anchor_lang::prelude::*;

/// Maximum fee in basis points (100% = 10000 bps)
pub const MAX_FEE_BPS: u16 = 10000;

/// Minimum deposit to prevent precision attacks (in token smallest unit)
pub const MIN_DEPOSIT_AMOUNT: u64 = 1000;

/// Calculate fee amount from gross amount
/// fee = amount * fee_bps / 10000
pub fn calculate_fee(amount: u64, fee_bps: u16) -> Option<u64> {
    (amount as u128)
        .checked_mul(fee_bps as u128)?
        .checked_div(MAX_FEE_BPS as u128)?
        .try_into()
        .ok()
}

/// Calculate net amount after fee deduction
pub fn calculate_net_after_fee(amount: u64, fee_bps: u16) -> Option<(u64, u64)> {
    let fee = calculate_fee(amount, fee_bps)?;
    let net = amount.checked_sub(fee)?;
    Some((net, fee))
}

/// Get current Unix timestamp from Clock sysvar
pub fn get_current_timestamp(clock: &Clock) -> i64 {
    clock.unix_timestamp
}

/// Check if a fee value is valid (0-10000 bps)
pub fn is_valid_fee(fee_bps: u16) -> bool {
    fee_bps <= MAX_FEE_BPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_fee() {
        // 20% of 1000 = 200
        assert_eq!(calculate_fee(1000, 2000), Some(200));

        // 0.5% of 10000 = 50
        assert_eq!(calculate_fee(10000, 50), Some(50));

        // 0% fee
        assert_eq!(calculate_fee(1000, 0), Some(0));

        // 100% fee
        assert_eq!(calculate_fee(1000, 10000), Some(1000));
    }

    #[test]
    fn test_calculate_net_after_fee() {
        // 20% fee on 1000 = 200 fee, 800 net
        assert_eq!(calculate_net_after_fee(1000, 2000), Some((800, 200)));

        // 0% fee
        assert_eq!(calculate_net_after_fee(1000, 0), Some((1000, 0)));
    }

    #[test]
    fn test_is_valid_fee() {
        assert!(is_valid_fee(0));
        assert!(is_valid_fee(5000));
        assert!(is_valid_fee(10000));
        assert!(!is_valid_fee(10001));
    }
}
