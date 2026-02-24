use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, InitializeAccount3, InitializeMint2, Mint, Token, TokenAccount};

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
    /// CHECK: PDA created and initialized in handler
    #[account(
        mut,
        seeds = [VaultPool::SHARES_MINT_SEED, token_mint.key().as_ref()],
        bump
    )]
    pub shares_mint: UncheckedAccount<'info>,

    /// Token account to hold vault's assets
    /// CHECK: PDA created and initialized in handler
    #[account(
        mut,
        seeds = [VaultPool::TOKEN_VAULT_SEED, token_mint.key().as_ref()],
        bump
    )]
    pub token_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreatePoolParams {
    pub daily_withdrawal_limit: u64,
}

pub fn create_pool_handler(ctx: Context<CreatePool>, params: CreatePoolParams) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let vault_pool = &mut ctx.accounts.vault_pool;

    // Create and initialize the shares mint PDA
    let shares_mint_bump = ctx.bumps.shares_mint;
    let token_mint_key = ctx.accounts.token_mint.key();
    let shares_mint_signer_seeds: &[&[&[u8]]] = &[&[
        VaultPool::SHARES_MINT_SEED,
        token_mint_key.as_ref(),
        &[shares_mint_bump],
    ]];

    let shares_mint_space = Mint::LEN;
    let shares_mint_lamports = Rent::get()?.minimum_balance(shares_mint_space);
    system_program::create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::CreateAccount {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.shares_mint.to_account_info(),
            },
            shares_mint_signer_seeds,
        ),
        shares_mint_lamports,
        shares_mint_space as u64,
        &anchor_spl::token::ID,
    )?;

    token::initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeMint2 {
                mint: ctx.accounts.shares_mint.to_account_info(),
            },
        ),
        ctx.accounts.token_mint.decimals,
        &vault_pool.key(),
        None,
    )?;

    // Create and initialize the token vault PDA
    let token_vault_bump = ctx.bumps.token_vault;
    let token_vault_signer_seeds: &[&[&[u8]]] = &[&[
        VaultPool::TOKEN_VAULT_SEED,
        token_mint_key.as_ref(),
        &[token_vault_bump],
    ]];

    let token_vault_space = TokenAccount::LEN;
    let token_vault_lamports = Rent::get()?.minimum_balance(token_vault_space);
    system_program::create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::CreateAccount {
                from: ctx.accounts.owner.to_account_info(),
                to: ctx.accounts.token_vault.to_account_info(),
            },
            token_vault_signer_seeds,
        ),
        token_vault_lamports,
        token_vault_space as u64,
        &anchor_spl::token::ID,
    )?;

    token::initialize_account3(CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        InitializeAccount3 {
            account: ctx.accounts.token_vault.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
            authority: vault_pool.to_account_info(),
        },
    ))?;

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
    config.total_pools = config
        .total_pools
        .checked_add(1)
        .ok_or(VaultError::MathOverflow)?;

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
