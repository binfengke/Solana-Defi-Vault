use anchor_lang::prelude::*;

/// User's pending withdrawal request (for queue mode)
#[account]
#[derive(InitSpace)]
pub struct WithdrawalRequest {
    /// The user who initiated the withdrawal
    pub user: Pubkey,

    /// The vault pool this request is for
    pub vault_pool: Pubkey,

    /// Amount of shares to redeem
    pub shares_amount: u64,

    /// Timestamp when request was created
    pub requested_at: i64,

    /// Whether this request has been processed
    pub is_processed: bool,

    /// PDA bump seed
    pub bump: u8,
}

impl WithdrawalRequest {
    pub const SEED_PREFIX: &'static [u8] = b"withdrawal_request";

    /// Check if request is still pending
    pub fn is_pending(&self) -> bool {
        !self.is_processed
    }
}
