use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::events::{Withdrawal as WithdrawalEvent, WithdrawalCancelled, WithdrawalRequested};
use crate::state::{VaultConfig, VaultPool, WithdrawalRequest};
use crate::utils::calculate_net_after_fee;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = !config.is_paused @ VaultError::VaultPaused,
        has_one = fee_receiver
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

    /// User's token account (destination)
    #[account(
        mut,
        constraint = user_token_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User's shares token account (source)
    #[account(
        mut,
        constraint = user_shares_account.mint == vault_pool.shares_mint @ VaultError::InvalidTokenMint
    )]
    pub user_shares_account: Account<'info, TokenAccount>,

    /// Fee receiver's token account
    #[account(
        mut,
        constraint = fee_receiver_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub fee_receiver_account: Account<'info, TokenAccount>,

    /// CHECK: Fee receiver from config
    pub fee_receiver: UncheckedAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub clock: Sysvar<'info, Clock>,
    pub token_program: Program<'info, Token>,
}

pub fn withdraw_handler(ctx: Context<Withdraw>, shares_amount: u64) -> Result<()> {
    // Validate withdrawal amount
    require!(shares_amount > 0, VaultError::ZeroWithdrawal);

    let vault_pool = &mut ctx.accounts.vault_pool;
    let config = &ctx.accounts.config;
    let clock = &ctx.accounts.clock;

    // Check user has enough shares
    require!(
        ctx.accounts.user_shares_account.amount >= shares_amount,
        VaultError::InsufficientShares
    );

    // Calculate assets to return
    let assets_to_return = vault_pool
        .calculate_assets_to_return(shares_amount)
        .ok_or(VaultError::InvalidSharesCalculation)?;

    // Check vault has enough liquidity
    require!(
        ctx.accounts.token_vault.amount >= assets_to_return,
        VaultError::InsufficientLiquidity
    );

    // Reset daily limit if new day
    if vault_pool.should_reset_daily_limit(clock.unix_timestamp) {
        vault_pool.withdrawn_today = 0;
        vault_pool.last_withdrawal_day = clock.unix_timestamp;
    }

    // Check daily withdrawal limit
    require!(
        !vault_pool.exceeds_daily_limit(assets_to_return),
        VaultError::DailyLimitExceeded
    );

    // Calculate fee
    let (net_amount, fee_amount) =
        calculate_net_after_fee(assets_to_return, config.withdrawal_fee_bps)
            .ok_or(VaultError::MathOverflow)?;

    // Create signer seeds for vault_pool PDA
    let token_mint = vault_pool.token_mint;
    let bump = vault_pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        VaultPool::SEED_PREFIX,
        token_mint.as_ref(),
        &[bump],
    ]];

    // Burn user's shares
    let burn_accounts = Burn {
        mint: ctx.accounts.shares_mint.to_account_info(),
        from: ctx.accounts.user_shares_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let burn_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), burn_accounts);
    token::burn(burn_ctx, shares_amount)?;

    // Transfer net amount to user
    let transfer_to_user = Transfer {
        from: ctx.accounts.token_vault.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: vault_pool.to_account_info(),
    };
    let transfer_user_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_to_user,
        signer_seeds,
    );
    token::transfer(transfer_user_ctx, net_amount)?;

    // Transfer fee to fee receiver
    if fee_amount > 0 {
        let transfer_fee = Transfer {
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.fee_receiver_account.to_account_info(),
            authority: vault_pool.to_account_info(),
        };
        let transfer_fee_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_fee,
            signer_seeds,
        );
        token::transfer(transfer_fee_ctx, fee_amount)?;
    }

    // Update vault state
    vault_pool.total_assets = vault_pool
        .total_assets
        .checked_sub(assets_to_return)
        .ok_or(VaultError::MathOverflow)?;
    vault_pool.total_shares = vault_pool
        .total_shares
        .checked_sub(shares_amount)
        .ok_or(VaultError::MathOverflow)?;
    vault_pool.withdrawn_today = vault_pool
        .withdrawn_today
        .checked_add(assets_to_return)
        .ok_or(VaultError::MathOverflow)?;

    emit!(WithdrawalEvent {
        user: ctx.accounts.user.key(),
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        shares_burned: shares_amount,
        assets_returned: assets_to_return,
        net_to_user: net_amount,
        fee_collected: fee_amount,
        total_assets: vault_pool.total_assets,
        total_shares: vault_pool.total_shares,
        timestamp: clock.unix_timestamp,
    });

    msg!("Withdrawal successful");
    msg!("User: {}", ctx.accounts.user.key());
    msg!("Shares burned: {}", shares_amount);
    msg!("Assets returned: {}", assets_to_return);
    msg!("Net to user: {}", net_amount);
    msg!("Fee collected: {}", fee_amount);

    Ok(())
}

// === Withdrawal Queue (for when liquidity is insufficient) ===

#[derive(Accounts)]
pub struct RequestWithdrawal<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = !config.is_paused @ VaultError::VaultPaused
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config,
        constraint = vault_pool.is_active @ VaultError::PoolNotActive
    )]
    pub vault_pool: Account<'info, VaultPool>,

    #[account(
        init,
        payer = user,
        space = 8 + WithdrawalRequest::INIT_SPACE,
        seeds = [
            WithdrawalRequest::SEED_PREFIX,
            vault_pool.key().as_ref(),
            user.key().as_ref()
        ],
        bump
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub clock: Sysvar<'info, Clock>,
    pub system_program: Program<'info, System>,
}

pub fn request_withdrawal_handler(
    ctx: Context<RequestWithdrawal>,
    shares_amount: u64,
) -> Result<()> {
    require!(shares_amount > 0, VaultError::ZeroWithdrawal);

    let withdrawal_request = &mut ctx.accounts.withdrawal_request;
    let clock = &ctx.accounts.clock;

    withdrawal_request.user = ctx.accounts.user.key();
    withdrawal_request.vault_pool = ctx.accounts.vault_pool.key();
    withdrawal_request.shares_amount = shares_amount;
    withdrawal_request.requested_at = clock.unix_timestamp;
    withdrawal_request.is_processed = false;
    withdrawal_request.bump = ctx.bumps.withdrawal_request;

    emit!(WithdrawalRequested {
        user: withdrawal_request.user,
        pool: withdrawal_request.vault_pool,
        request: withdrawal_request.key(),
        shares_amount,
        timestamp: clock.unix_timestamp,
    });

    msg!("Withdrawal request created");
    msg!("User: {}", withdrawal_request.user);
    msg!("Shares amount: {}", shares_amount);
    msg!("Requested at: {}", withdrawal_request.requested_at);

    Ok(())
}

#[derive(Accounts)]
pub struct CancelWithdrawal<'info> {
    #[account(
        mut,
        seeds = [
            WithdrawalRequest::SEED_PREFIX,
            withdrawal_request.vault_pool.as_ref(),
            user.key().as_ref()
        ],
        bump = withdrawal_request.bump,
        has_one = user,
        constraint = !withdrawal_request.is_processed @ VaultError::WithdrawalAlreadyProcessed,
        close = user
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    #[account(mut)]
    pub user: Signer<'info>,
}

pub fn cancel_withdrawal_handler(ctx: Context<CancelWithdrawal>) -> Result<()> {
    let withdrawal_request = &ctx.accounts.withdrawal_request;

    emit!(WithdrawalCancelled {
        user: ctx.accounts.user.key(),
        pool: withdrawal_request.vault_pool,
        request: withdrawal_request.key(),
        shares_amount: withdrawal_request.shares_amount,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!(
        "Withdrawal request cancelled for user: {}",
        ctx.accounts.user.key()
    );
    Ok(())
}
