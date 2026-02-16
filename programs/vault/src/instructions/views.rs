use anchor_lang::prelude::*;

use crate::state::{VaultConfig, VaultPool};

/// View helper for calculating deposit preview
#[derive(Accounts)]
pub struct PreviewDeposit<'info> {
    #[account(
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump
    )]
    pub vault_pool: Account<'info, VaultPool>,
}

/// View helper for calculating withdrawal preview
#[derive(Accounts)]
pub struct PreviewWithdraw<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump
    )]
    pub vault_pool: Account<'info, VaultPool>,
}

/// Result of deposit preview
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DepositPreview {
    /// Amount of shares that would be minted
    pub shares_to_mint: u64,
    /// Current share price (scaled by 1e9 for precision)
    pub share_price_scaled: u64,
    /// Total assets after deposit
    pub new_total_assets: u64,
    /// Total shares after deposit
    pub new_total_shares: u64,
}

/// Result of withdrawal preview
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WithdrawPreview {
    /// Gross assets to be returned (before fees)
    pub gross_assets: u64,
    /// Fee amount to be deducted
    pub fee_amount: u64,
    /// Net assets user will receive
    pub net_assets: u64,
    /// Current share price (scaled by 1e9 for precision)
    pub share_price_scaled: u64,
}

/// Result of pool info query
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PoolInfoView {
    pub token_mint: Pubkey,
    pub shares_mint: Pubkey,
    pub total_assets: u64,
    pub total_shares: u64,
    pub share_price_scaled: u64,
    pub daily_withdrawal_limit: u64,
    pub withdrawn_today: u64,
    pub is_active: bool,
}

const PRICE_SCALE: u128 = 1_000_000_000; // 1e9 for precision

/// Calculate shares to mint for a deposit amount
pub fn preview_deposit_handler(ctx: Context<PreviewDeposit>, amount: u64) -> Result<DepositPreview> {
    let pool = &ctx.accounts.vault_pool;

    let shares_to_mint = pool
        .calculate_shares_to_mint(amount)
        .unwrap_or(0);

    let share_price_scaled = calculate_share_price_scaled(pool.total_assets, pool.total_shares);

    Ok(DepositPreview {
        shares_to_mint,
        share_price_scaled,
        new_total_assets: pool.total_assets.saturating_add(amount),
        new_total_shares: pool.total_shares.saturating_add(shares_to_mint),
    })
}

/// Calculate assets to return for a withdrawal
pub fn preview_withdraw_handler(
    ctx: Context<PreviewWithdraw>,
    shares_amount: u64,
) -> Result<WithdrawPreview> {
    let config = &ctx.accounts.config;
    let pool = &ctx.accounts.vault_pool;

    let gross_assets = pool
        .calculate_assets_to_return(shares_amount)
        .unwrap_or(0);

    let fee_amount = (gross_assets as u128)
        .checked_mul(config.withdrawal_fee_bps as u128)
        .and_then(|v| v.checked_div(10000))
        .map(|v| v as u64)
        .unwrap_or(0);

    let net_assets = gross_assets.saturating_sub(fee_amount);
    let share_price_scaled = calculate_share_price_scaled(pool.total_assets, pool.total_shares);

    Ok(WithdrawPreview {
        gross_assets,
        fee_amount,
        net_assets,
        share_price_scaled,
    })
}

/// Get pool information
pub fn get_pool_info_handler(ctx: Context<PreviewDeposit>) -> Result<PoolInfoView> {
    let pool = &ctx.accounts.vault_pool;
    let share_price_scaled = calculate_share_price_scaled(pool.total_assets, pool.total_shares);

    Ok(PoolInfoView {
        token_mint: pool.token_mint,
        shares_mint: pool.shares_mint,
        total_assets: pool.total_assets,
        total_shares: pool.total_shares,
        share_price_scaled,
        daily_withdrawal_limit: pool.daily_withdrawal_limit,
        withdrawn_today: pool.withdrawn_today,
        is_active: pool.is_active,
    })
}

/// Calculate share price scaled by 1e9
/// Returns 1e9 if pool is empty (1:1 ratio)
fn calculate_share_price_scaled(total_assets: u64, total_shares: u64) -> u64 {
    if total_shares == 0 {
        return PRICE_SCALE as u64; // 1:1 ratio
    }

    (total_assets as u128)
        .checked_mul(PRICE_SCALE)
        .and_then(|v| v.checked_div(total_shares as u128))
        .map(|v| v as u64)
        .unwrap_or(PRICE_SCALE as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_share_price_scaled() {
        // Empty pool = 1:1
        assert_eq!(calculate_share_price_scaled(0, 0), 1_000_000_000);

        // Equal assets and shares = 1:1
        assert_eq!(calculate_share_price_scaled(1000, 1000), 1_000_000_000);

        // 2000 assets, 1000 shares = 2:1
        assert_eq!(calculate_share_price_scaled(2000, 1000), 2_000_000_000);

        // 500 assets, 1000 shares = 0.5:1
        assert_eq!(calculate_share_price_scaled(500, 1000), 500_000_000);
    }
}
