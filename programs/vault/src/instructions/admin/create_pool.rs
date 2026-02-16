use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::errors::VaultError;
use crate::events::PoolCreated;
use crate::state::{VaultConfig, VaultPool};

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(
        init,
        payer = owner,
        space = 8 + VaultPool::INIT_SPACE,
        seeds = [VaultPool::SEED_PREFIX, token_mint.key().as_ref()],
        bump
    )]
    pub vault_pool: Box<Account<'info, VaultPool>>,

    /// The underlying token mint (e.g., USDC)
    pub token_mint: Box<Account<'info, Mint>>,

    /// The shares token mint (e.g., vUSDC) - created by this instruction
    #[account(
        init,
        payer = owner,
        seeds = [VaultPool::SHARES_MINT_SEED, token_mint.key().as_ref()],
        bump,
        mint::decimals = token_mint.decimals,
        mint::authority = vault_pool,
    )]
    pub shares_mint: Box<Account<'info, Mint>>,

    /// Token account to hold vault's assets
    #[account(
        init,
        payer = owner,
        seeds = [VaultPool::TOKEN_VAULT_SEED, token_mint.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_pool,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreatePoolParams {
    pub daily_withdrawal_limit: u64,
}

pub fn create_pool_handler(ctx: Context<CreatePool>, params: CreatePoolParams) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let vault_pool = &mut ctx.accounts.vault_pool;

    // Set pool data
    vault_pool.config = config.key();
    vault_pool.token_mint = ctx.accounts.token_mint.key();
    vault_pool.shares_mint = ctx.accounts.shares_mint.key();
    vault_pool.token_vault = ctx.accounts.token_vault.key();
    vault_pool.total_assets = 0;
    vault_pool.total_shares = 0;
    vault_pool.daily_withdrawal_limit = params.daily_withdrawal_limit;
    vault_pool.withdrawn_today = 0;
    vault_pool.last_withdrawal_day = 0;
    vault_pool.is_active = true;
    vault_pool.pool_index = config.total_pools;
    vault_pool.bump = ctx.bumps.vault_pool;
    vault_pool.shares_mint_bump = ctx.bumps.shares_mint;

    // Increment pool count
    config.total_pools = config.total_pools.checked_add(1).unwrap();

    emit!(PoolCreated {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        shares_mint: vault_pool.shares_mint,
        token_vault: vault_pool.token_vault,
        pool_index: vault_pool.pool_index,
        daily_withdrawal_limit: vault_pool.daily_withdrawal_limit,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Vault pool created");
    msg!("Token mint: {}", vault_pool.token_mint);
    msg!("Shares mint: {}", vault_pool.shares_mint);
    msg!("Pool index: {}", vault_pool.pool_index);
    msg!("Daily withdrawal limit: {}", vault_pool.daily_withdrawal_limit);

    Ok(())
}
