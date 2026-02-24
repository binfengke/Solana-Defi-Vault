use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::events::WithdrawalProcessed;
use crate::state::{VaultConfig, VaultPool, WithdrawalRequest};
use crate::utils::calculate_net_after_fee;

#[derive(Accounts)]
pub struct ProcessWithdrawalQueue<'info> {
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
        has_one = token_vault,
        has_one = shares_mint
    )]
    pub vault_pool: Account<'info, VaultPool>,

    /// Vault's token account
    #[account(mut)]
    pub token_vault: Account<'info, TokenAccount>,

    /// Shares token mint
    #[account(mut)]
    pub shares_mint: Account<'info, Mint>,

    /// The withdrawal request to process
    #[account(
        mut,
        seeds = [
            WithdrawalRequest::SEED_PREFIX,
            vault_pool.key().as_ref(),
            withdrawal_request.user.as_ref()
        ],
        bump = withdrawal_request.bump,
        has_one = vault_pool,
        constraint = !withdrawal_request.is_processed @ VaultError::WithdrawalAlreadyProcessed
    )]
    pub withdrawal_request: Account<'info, WithdrawalRequest>,

    /// User's token account (destination)
    #[account(
        mut,
        constraint = user_token_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint,
        constraint = user_token_account.owner == withdrawal_request.user
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User's shares token account
    #[account(
        mut,
        constraint = user_shares_account.mint == vault_pool.shares_mint @ VaultError::InvalidTokenMint,
        constraint = user_shares_account.owner == withdrawal_request.user
    )]
    pub user_shares_account: Account<'info, TokenAccount>,

    /// Fee receiver's token account
    #[account(
        mut,
        constraint = fee_receiver_account.mint == vault_pool.token_mint @ VaultError::InvalidTokenMint,
        constraint = fee_receiver_account.owner == fee_receiver.key() @ VaultError::InvalidFeeReceiverAccount
    )]
    pub fee_receiver_account: Account<'info, TokenAccount>,

    /// CHECK: Fee receiver from config
    pub fee_receiver: UncheckedAccount<'info>,

    /// CHECK: The user who made the request (for verification)
    #[account(constraint = user.key() == withdrawal_request.user @ VaultError::UserMismatch)]
    pub user: UncheckedAccount<'info>,

    /// Operator or Owner
    pub authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn process_withdrawal_queue_handler(ctx: Context<ProcessWithdrawalQueue>) -> Result<()> {
    let config = &ctx.accounts.config;
    let vault_pool = &mut ctx.accounts.vault_pool;
    let withdrawal_request = &mut ctx.accounts.withdrawal_request;

    require_keys_eq!(
        ctx.accounts.user.key(),
        withdrawal_request.user,
        VaultError::UserMismatch
    );

    let shares_amount = withdrawal_request.shares_amount;

    // Check user still has the shares
    require!(
        ctx.accounts.user_shares_account.amount >= shares_amount,
        VaultError::InsufficientShares
    );

    // Calculate assets to return
    let assets_to_return = vault_pool
        .calculate_assets_to_return(shares_amount)
        .ok_or(VaultError::InvalidSharesCalculation)?;

    // Check vault has enough liquidity now
    require!(
        ctx.accounts.token_vault.amount >= assets_to_return,
        VaultError::InsufficientLiquidity
    );

    // Calculate fee
    let (net_amount, fee_amount) =
        calculate_net_after_fee(assets_to_return, config.withdrawal_fee_bps)
            .ok_or(VaultError::MathOverflow)?;

    // Create signer seeds for vault_pool PDA
    let token_mint = vault_pool.token_mint;
    let bump = vault_pool.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[VaultPool::SEED_PREFIX, token_mint.as_ref(), &[bump]]];

    // Burn user's shares (requires user to have delegated or we use a different approach)
    // For queue processing, the user must have approved the vault_pool to burn their shares
    let burn_accounts = Burn {
        mint: ctx.accounts.shares_mint.to_account_info(),
        from: ctx.accounts.user_shares_account.to_account_info(),
        authority: vault_pool.to_account_info(),
    };
    let burn_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        burn_accounts,
        signer_seeds,
    );
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

    // Mark request as processed
    withdrawal_request.is_processed = true;

    emit!(WithdrawalProcessed {
        user: withdrawal_request.user,
        pool: vault_pool.key(),
        request: withdrawal_request.key(),
        shares_burned: shares_amount,
        assets_returned: assets_to_return,
        net_to_user: net_amount,
        fee_collected: fee_amount,
        operator: ctx.accounts.authority.key(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Withdrawal request processed");
    msg!("User: {}", withdrawal_request.user);
    msg!("Shares burned: {}", shares_amount);
    msg!("Assets returned: {}", assets_to_return);
    msg!("Net to user: {}", net_amount);
    msg!("Fee collected: {}", fee_amount);

    Ok(())
}
