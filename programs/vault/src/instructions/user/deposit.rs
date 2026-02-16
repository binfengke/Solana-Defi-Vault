use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::events::Deposit as DepositEvent;
use crate::state::{VaultConfig, VaultPool};
use crate::utils::MIN_DEPOSIT_AMOUNT;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = !config.is_paused @ VaultError::VaultPaused
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config,
        has_one = token_vault,
        has_one = shares_mint,
        constraint = vault_pool.is_active @ VaultError::PoolNotActive
    )]
    pub vault_pool: Account<'info, VaultPool>,

    /// Vault's token account
    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Shares token mint
    #[account(mut)]
    pub shares_mint: Account<'info, Mint>,

    /// User's token account (source)
    #[account(
        mut,
        constraint = user_token_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User's shares token account (destination)
    #[account(
        mut,
        constraint = user_shares_account.mint == vault_pool.shares_mint @ VaultError::InvalidTokenMint
    )]
    pub user_shares_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn deposit_handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    // Validate deposit amount
    require!(amount > 0, VaultError::ZeroDeposit);
    require!(amount >= MIN_DEPOSIT_AMOUNT, VaultError::MinimumDepositNotMet);

    let vault_pool = &mut ctx.accounts.vault_pool;

    // Calculate shares to mint
    let shares_to_mint = vault_pool
        .calculate_shares_to_mint(amount)
        .ok_or(VaultError::InvalidSharesCalculation)?;

    require!(shares_to_mint > 0, VaultError::InvalidSharesCalculation);

    // Transfer tokens from user to vault
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    token::transfer(cpi_ctx, amount)?;

    // Mint shares to user
    let token_mint = vault_pool.token_mint;
    let bump = vault_pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        VaultPool::SEED_PREFIX,
        token_mint.as_ref(),
        &[bump],
    ]];

    let mint_accounts = MintTo {
        mint: ctx.accounts.shares_mint.to_account_info(),
        to: ctx.accounts.user_shares_account.to_account_info(),
        authority: vault_pool.to_account_info(),
    };
    let mint_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        mint_accounts,
        signer_seeds,
    );

    token::mint_to(mint_ctx, shares_to_mint)?;

    // Update vault state
    vault_pool.total_assets = vault_pool
        .total_assets
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;
    vault_pool.total_shares = vault_pool
        .total_shares
        .checked_add(shares_to_mint)
        .ok_or(VaultError::MathOverflow)?;

    emit!(DepositEvent {
        user: ctx.accounts.user.key(),
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        amount,
        shares_minted: shares_to_mint,
        total_assets: vault_pool.total_assets,
        total_shares: vault_pool.total_shares,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Deposit successful");
    msg!("User: {}", ctx.accounts.user.key());
    msg!("Amount deposited: {}", amount);
    msg!("Shares minted: {}", shares_to_mint);
    msg!("Total assets: {}", vault_pool.total_assets);
    msg!("Total shares: {}", vault_pool.total_shares);

    Ok(())
}
