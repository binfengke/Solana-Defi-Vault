use anchor_lang::prelude::*;

/// Individual Vault Pool for a specific SPL Token
#[account]
#[derive(InitSpace)]
pub struct VaultPool {
    /// Reference to the global VaultConfig
    pub config: Pubkey,

    /// The underlying SPL Token mint (e.g., USDC)
    pub token_mint: Pubkey,

    /// The Vault Shares token mint (e.g., vUSDC)
    pub shares_mint: Pubkey,

    /// Token account that holds the vault's assets
    pub token_vault: Pubkey,

    /// Total assets under management (includes deployed + idle funds)
    pub total_assets: u64,

    /// Total shares minted
    pub total_shares: u64,

    /// Daily withdrawal limit (0 = unlimited)
    pub daily_withdrawal_limit: u64,

    /// Amount withdrawn today (resets daily)
    pub withdrawn_today: u64,

    /// Timestamp of last withdrawal day (for daily reset)
    pub last_withdrawal_day: i64,

    /// Whether this pool is active (accepting deposits/withdrawals)
    pub is_active: bool,

    /// Pool index (for PDA derivation)
    pub pool_index: u64,

    /// PDA bump seed
    pub bump: u8,

    /// Shares mint bump seed
    pub shares_mint_bump: u8,
}

impl VaultPool {
    pub const SEED_PREFIX: &'static [u8] = b"vault_pool";
    pub const SHARES_MINT_SEED: &'static [u8] = b"shares_mint";
    pub const TOKEN_VAULT_SEED: &'static [u8] = b"token_vault";

    /// Seconds in a day for withdrawal limit reset
    pub const SECONDS_PER_DAY: i64 = 86400;

    /// Calculate shares to mint for a deposit
    /// Formula: shares = deposit_amount * total_shares / total_assets
    /// For first deposit: 1:1 ratio
    pub fn calculate_shares_to_mint(&self, deposit_amount: u64) -> Option<u64> {
        if self.total_shares == 0 || self.total_assets == 0 {
            // First deposit: 1:1 ratio
            Some(deposit_amount)
        } else {
            // shares = deposit_amount * total_shares / total_assets
            (deposit_amount as u128)
                .checked_mul(self.total_shares as u128)?
                .checked_div(self.total_assets as u128)?
                .try_into()
                .ok()
        }
    }

    /// Calculate assets to return for a withdrawal
    /// Formula: assets = shares_amount * total_assets / total_shares
    pub fn calculate_assets_to_return(&self, shares_amount: u64) -> Option<u64> {
        if self.total_shares == 0 {
            return None;
        }

        (shares_amount as u128)
            .checked_mul(self.total_assets as u128)?
            .checked_div(self.total_shares as u128)?
            .try_into()
            .ok()
    }

    /// Check if daily withdrawal limit needs reset
    pub fn should_reset_daily_limit(&self, current_timestamp: i64) -> bool {
        let current_day = current_timestamp / Self::SECONDS_PER_DAY;
        let last_day = self.last_withdrawal_day / Self::SECONDS_PER_DAY;
        current_day > last_day
    }

    /// Check if withdrawal amount exceeds daily limit
    pub fn exceeds_daily_limit(&self, amount: u64) -> bool {
        if self.daily_withdrawal_limit == 0 {
            return false; // No limit
        }
        self.withdrawn_today.saturating_add(amount) > self.daily_withdrawal_limit
    }

    /// Get available liquidity in the vault
    pub fn available_liquidity(&self) -> u64 {
        self.total_assets
    }
}
