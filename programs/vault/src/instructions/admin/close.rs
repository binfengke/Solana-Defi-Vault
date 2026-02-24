use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{VaultConfig, VaultPool, WithdrawalRequest};

/// Close a processed withdrawal request and reclaim rent
#[derive(Accounts)]
pub struct CloseWithdrawalRequest<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.is_authorized(&authority.key()) @ VaultError::Unauthorized
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [
            WithdrawalRequest::SEED_PREFIX,
            withdrawal_request.vault_pool.as_ref(),
            withdrawal_request.user.as_ref()
        ],
        bump = withdrawal_request.bump,
        constraint = withdrawal_request.is_processed @ VaultError::WithdrawalRequestNotFound,
        close = user
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    /// CHECK: Rent always returned to the withdrawal request user
    #[account(mut, address = withdrawal_request.user @ VaultError::UserMismatch)]
    pub user: UncheckedAccount<'info>,

    /// Operator or Owner
    pub authority: Signer<'info>,
}

pub fn close_withdrawal_request_handler(_ctx: Context<CloseWithdrawalRequest>) -> Result<()> {
    msg!("Processed withdrawal request closed, rent reclaimed");
    Ok(())
}

/// Close an empty and inactive vault pool (Owner only)
#[derive(Accounts)]
pub struct ClosePool<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(
        mut,
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config,
        constraint = !vault_pool.is_active @ VaultError::PoolAlreadyActive,
        constraint = vault_pool.total_shares == 0 @ VaultError::InsufficientLiquidity,
        constraint = vault_pool.total_assets == 0 @ VaultError::InsufficientLiquidity,
        close = owner
    )]
    pub vault_pool: Box<Account<'info, VaultPool>>,

    #[account(mut)]
    pub owner: Signer<'info>,
}

pub fn close_pool_handler(ctx: Context<ClosePool>) -> Result<()> {
    let config = &mut ctx.accounts.config;

    // Decrement pool count
    config.total_pools = config.total_pools.saturating_sub(1);

    msg!("Vault pool closed");
    msg!("Token mint: {}", ctx.accounts.vault_pool.token_mint);
    msg!("Remaining pools: {}", config.total_pools);

    Ok(())
}
