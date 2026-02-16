use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::events::VaultInitialized;
use crate::state::VaultConfig;
use crate::utils::is_valid_fee;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [VaultConfig::SEED_PREFIX],
        bump
    )]
    pub config: Account<'info, VaultConfig>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeParams {
    pub operator: Pubkey,
    pub fee_receiver: Pubkey,
    pub performance_fee_bps: u16,
    pub withdrawal_fee_bps: u16,
}

pub fn initialize_handler(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
    // Validate fees
    require!(
        is_valid_fee(params.performance_fee_bps),
        VaultError::InvalidFee
    );
    require!(
        is_valid_fee(params.withdrawal_fee_bps),
        VaultError::InvalidFee
    );

    let config = &mut ctx.accounts.config;

    config.owner = ctx.accounts.owner.key();
    config.operator = params.operator;
    config.is_paused = false;
    config.performance_fee_bps = params.performance_fee_bps;
    config.withdrawal_fee_bps = params.withdrawal_fee_bps;
    config.fee_receiver = params.fee_receiver;
    config.total_pools = 0;
    config.bump = ctx.bumps.config;

    emit!(VaultInitialized {
        owner: config.owner,
        operator: config.operator,
        fee_receiver: config.fee_receiver,
        performance_fee_bps: config.performance_fee_bps,
        withdrawal_fee_bps: config.withdrawal_fee_bps,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("Vault config initialized");
    msg!("Owner: {}", config.owner);
    msg!("Operator: {}", config.operator);
    msg!("Performance fee: {} bps", config.performance_fee_bps);
    msg!("Withdrawal fee: {} bps", config.withdrawal_fee_bps);

    Ok(())
}
