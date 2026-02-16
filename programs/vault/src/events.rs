use anchor_lang::prelude::*;

// ============ Admin Events ============

#[event]
pub struct VaultInitialized {
    pub owner: Pubkey,
    pub operator: Pubkey,
    pub fee_receiver: Pubkey,
    pub performance_fee_bps: u16,
    pub withdrawal_fee_bps: u16,
    pub timestamp: i64,
}

#[event]
pub struct ConfigUpdated {
    pub owner: Pubkey,
    pub old_operator: Option<Pubkey>,
    pub new_operator: Option<Pubkey>,
    pub old_fee_receiver: Option<Pubkey>,
    pub new_fee_receiver: Option<Pubkey>,
    pub old_performance_fee_bps: Option<u16>,
    pub new_performance_fee_bps: Option<u16>,
    pub old_withdrawal_fee_bps: Option<u16>,
    pub new_withdrawal_fee_bps: Option<u16>,
    pub timestamp: i64,
}

#[event]
pub struct PoolCreated {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub shares_mint: Pubkey,
    pub token_vault: Pubkey,
    pub pool_index: u64,
    pub daily_withdrawal_limit: u64,
    pub timestamp: i64,
}

#[event]
pub struct OwnershipTransferred {
    pub old_owner: Pubkey,
    pub new_owner: Pubkey,
    pub timestamp: i64,
}

// ============ Emergency Events ============

#[event]
pub struct VaultPaused {
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct VaultUnpaused {
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PoolStatusChanged {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub is_active: bool,
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawalLimitUpdated {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub old_limit: u64,
    pub new_limit: u64,
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct EmergencyWithdrawal {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}

// ============ User Events ============

#[event]
pub struct Deposit {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub shares_minted: u64,
    pub total_assets: u64,
    pub total_shares: u64,
    pub timestamp: i64,
}

#[event]
pub struct Withdrawal {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub shares_burned: u64,
    pub assets_returned: u64,
    pub net_to_user: u64,
    pub fee_collected: u64,
    pub total_assets: u64,
    pub total_shares: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawalRequested {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub request: Pubkey,
    pub shares_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawalCancelled {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub request: Pubkey,
    pub shares_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct WithdrawalProcessed {
    pub user: Pubkey,
    pub pool: Pubkey,
    pub request: Pubkey,
    pub shares_burned: u64,
    pub assets_returned: u64,
    pub net_to_user: u64,
    pub fee_collected: u64,
    pub operator: Pubkey,
    pub timestamp: i64,
}

// ============ Operator Events ============

#[event]
pub struct YieldInjected {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub gross_yield: u64,
    pub performance_fee: u64,
    pub net_yield: u64,
    pub new_total_assets: u64,
    pub operator: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct StrategyWithdrawal {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub operator: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct StrategyReturn {
    pub pool: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub source: Pubkey,
    pub operator: Pubkey,
    pub timestamp: i64,
}
