use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::events::{
    EmergencyWithdrawal, OwnershipTransferred, PoolStatusChanged, VaultPaused, VaultUnpaused,
    WithdrawalLimitUpdated,
};
use crate::state::{VaultConfig, VaultPool};

#[derive(Accounts)]
pub struct Pause<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    pub owner: Signer<'info>,
}

pub fn pause_handler(ctx: Context<Pause>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.is_paused = true;

    emit!(VaultPaused {
        owner: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Vault paused by owner: {}", ctx.accounts.owner.key());
    Ok(())
}

pub fn unpause_handler(ctx: Context<Pause>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.is_paused = false;

    emit!(VaultUnpaused {
        owner: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Vault unpaused by owner: {}", ctx.accounts.owner.key());
    Ok(())
}

#[derive(Accounts)]
pub struct SetPoolStatus<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config
    )]
    pub vault_pool: Account<'info, VaultPool>,

    pub owner: Signer<'info>,
}

pub fn set_pool_active_handler(ctx: Context<SetPoolStatus>, is_active: bool) -> Result<()> {
    let vault_pool = &mut ctx.accounts.vault_pool;
    vault_pool.is_active = is_active;

    emit!(PoolStatusChanged {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        is_active,
        owner: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!(
        "Pool {} status set to: {}",
        vault_pool.token_mint,
        if is_active { "active" } else { "inactive" }
    );
    Ok(())
}

#[derive(Accounts)]
pub struct EmergencyWithdraw<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config,
        has_one = token_vault
    )]
    pub vault_pool: Account<'info, VaultPool>,

    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Destination for emergency withdrawal
    #[account(
        mut,
        constraint = destination.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub destination: Account<'info, TokenAccount>,

    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn emergency_withdraw_handler(ctx: Context<EmergencyWithdraw>) -> Result<()> {
    let vault_pool = &mut ctx.accounts.vault_pool;
    let config = &mut ctx.accounts.config;

    require_keys_eq!(
        ctx.accounts.destination.mint,
        vault_pool.token_mint,
        VaultError::InvalidTokenMint
    );

    let amount = ctx.accounts.token_vault.amount;

    if amount == 0 {
        msg!("No funds to withdraw");
        return Ok(());
    }

    // Create signer seeds for vault_pool PDA
    let token_mint = vault_pool.token_mint;
    let bump = vault_pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        VaultPool::SEED_PREFIX,
        token_mint.as_ref(),
        &[bump],
    ]];

    // Transfer all funds to destination
    let cpi_accounts = Transfer {
        from: ctx.accounts.token_vault.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: vault_pool.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

    token::transfer(cpi_ctx, amount)?;

    // Update state
    vault_pool.total_assets = 0;
    vault_pool.is_active = false;

    // Auto-pause the vault
    config.is_paused = true;

    emit!(EmergencyWithdrawal {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        amount,
        destination: ctx.accounts.destination.key(),
        owner: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Emergency withdrawal executed");
    msg!("Amount: {}", amount);
    msg!("Destination: {}", ctx.accounts.destination.key());
    msg!("Vault is now paused");

    Ok(())
}

#[derive(Accounts)]
pub struct TransferOwnership<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    pub owner: Signer<'info>,

    /// CHECK: New owner address
    pub new_owner: UncheckedAccount<'info>,
}

pub fn transfer_ownership_handler(ctx: Context<TransferOwnership>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let old_owner = config.owner;

    config.owner = ctx.accounts.new_owner.key();

    emit!(OwnershipTransferred {
        old_owner,
        new_owner: config.owner,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Ownership transferred");
    msg!("Old owner: {}", old_owner);
    msg!("New owner: {}", config.owner);

    Ok(())
}

#[derive(Accounts)]
pub struct SetWithdrawalLimit<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config
    )]
    pub vault_pool: Account<'info, VaultPool>,

    pub owner: Signer<'info>,
}

pub fn set_withdrawal_limit_handler(
    ctx: Context<SetWithdrawalLimit>,
    daily_limit: u64,
) -> Result<()> {
    let vault_pool = &mut ctx.accounts.vault_pool;
    let old_limit = vault_pool.daily_withdrawal_limit;
    vault_pool.daily_withdrawal_limit = daily_limit;

    emit!(WithdrawalLimitUpdated {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        old_limit,
        new_limit: daily_limit,
        owner: ctx.accounts.owner.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!(
        "Daily withdrawal limit set to: {} for pool {}",
        daily_limit,
        vault_pool.token_mint
    );

    Ok(())
}
