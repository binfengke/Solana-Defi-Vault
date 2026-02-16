use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::events::{StrategyReturn, StrategyWithdrawal, YieldInjected};
use crate::state::{VaultConfig, VaultPool};
use crate::utils::calculate_fee;

#[derive(Accounts)]
pub struct InjectYield<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.is_authorized(&authority.key()) @ VaultError::Unauthorized,
        has_one = fee_receiver
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

    /// Vault's token account
    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Source of yield (operator's token account)
    #[account(
        mut,
        constraint = yield_source.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub yield_source: Account<'info, TokenAccount>,

    /// Fee receiver's token account
    #[account(
        mut,
        constraint = fee_receiver_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub fee_receiver_account: Account<'info, TokenAccount>,

    /// CHECK: Fee receiver from config
    pub fee_receiver: UncheckedAccount<'info>,

    /// Operator or Owner
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn inject_yield_handler(ctx: Context<InjectYield>, yield_amount: u64) -> Result<()> {
    require!(yield_amount > 0, VaultError::ZeroDeposit);

    let config = &ctx.accounts.config;
    let vault_pool = &mut ctx.accounts.vault_pool;

    // Calculate performance fee
    let fee_amount = calculate_fee(yield_amount, config.performance_fee_bps)
        .ok_or(VaultError::MathOverflow)?;
    let net_yield = yield_amount
        .checked_sub(fee_amount)
        .ok_or(VaultError::MathOverflow)?;

    // Transfer net yield to vault
    let transfer_yield = Transfer {
        from: ctx.accounts.yield_source.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let transfer_yield_ctx =
        CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_yield);
    token::transfer(transfer_yield_ctx, net_yield)?;

    // Transfer fee to fee receiver
    if fee_amount > 0 {
        let transfer_fee = Transfer {
            from: ctx.accounts.yield_source.to_account_info(),
            to: ctx.accounts.fee_receiver_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };
        let transfer_fee_ctx =
            CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_fee);
        token::transfer(transfer_fee_ctx, fee_amount)?;
    }

    // Update total assets (auto-compounding)
    vault_pool.total_assets = vault_pool
        .total_assets
        .checked_add(net_yield)
        .ok_or(VaultError::MathOverflow)?;

    emit!(YieldInjected {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        gross_yield: yield_amount,
        performance_fee: fee_amount,
        net_yield,
        new_total_assets: vault_pool.total_assets,
        operator: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Yield injected successfully");
    msg!("Gross yield: {}", yield_amount);
    msg!("Performance fee: {}", fee_amount);
    msg!("Net yield added: {}", net_yield);
    msg!("New total assets: {}", vault_pool.total_assets);

    Ok(())
}

// === Strategy Fund Management ===

#[derive(Accounts)]
pub struct WithdrawForStrategy<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.is_authorized(&authority.key()) @ VaultError::Unauthorized
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

    /// Vault's token account
    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Destination for strategy deployment
    #[account(
        mut,
        constraint = strategy_destination.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub strategy_destination: Account<'info, TokenAccount>,

    /// Operator or Owner
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn withdraw_for_strategy_handler(
    ctx: Context<WithdrawForStrategy>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, VaultError::ZeroWithdrawal);

    let vault_pool = &ctx.accounts.vault_pool;

    // Check vault has enough liquidity
    require!(
        ctx.accounts.token_vault.amount >= amount,
        VaultError::InsufficientLiquidity
    );

    // Create signer seeds for vault_pool PDA
    let token_mint = vault_pool.token_mint;
    let bump = vault_pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[VaultPool::SEED_PREFIX, token_mint.as_ref(), &[bump]]];

    // Transfer to strategy destination
    let transfer_accounts = Transfer {
        from: ctx.accounts.token_vault.to_account_info(),
        to: ctx.accounts.strategy_destination.to_account_info(),
        authority: ctx.accounts.vault_pool.to_account_info(),
    };
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
        signer_seeds,
    );
    token::transfer(transfer_ctx, amount)?;

    // Note: total_assets remains unchanged because funds are still under management
    // They're just deployed externally

    emit!(StrategyWithdrawal {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        amount,
        destination: ctx.accounts.strategy_destination.key(),
        operator: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Funds withdrawn for strategy");
    msg!("Amount: {}", amount);
    msg!("Destination: {}", ctx.accounts.strategy_destination.key());

    Ok(())
}

#[derive(Accounts)]
pub struct ReturnFromStrategy<'info> {
    #[account(
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        constraint = config.is_authorized(&authority.key()) @ VaultError::Unauthorized
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(
        seeds = [VaultPool::SEED_PREFIX, vault_pool.token_mint.as_ref()],
        bump = vault_pool.bump,
        has_one = config,
        has_one = token_vault
    )]
    pub vault_pool: Account<'info, VaultPool>,

    /// Vault's token account
    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Source of returning funds
    #[account(
        mut,
        constraint = strategy_source.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint
    )]
    pub strategy_source: Account<'info, TokenAccount>,

    /// Operator or Owner
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn return_from_strategy_handler(ctx: Context<ReturnFromStrategy>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroDeposit);

    let vault_pool = &ctx.accounts.vault_pool;

    // Transfer back to vault
    let transfer_accounts = Transfer {
        from: ctx.accounts.strategy_source.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let transfer_ctx =
        CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_accounts);
    token::transfer(transfer_ctx, amount)?;

    // Note: total_assets remains unchanged because this is just returning deployed funds

    emit!(StrategyReturn {
        pool: vault_pool.key(),
        token_mint: vault_pool.token_mint,
        amount,
        source: ctx.accounts.strategy_source.key(),
        operator: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Funds returned from strategy");
    msg!("Amount: {}", amount);

    Ok(())
}
