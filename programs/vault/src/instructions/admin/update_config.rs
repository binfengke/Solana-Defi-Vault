use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::VaultConfig;
use crate::utils::is_valid_fee;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [VaultConfig::SEED_PREFIX],
        bump = config.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub config: Account<'info, VaultConfig>,

    pub owner: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateConfigParams {
    pub new_operator: Option<Pubkey>,
    pub new_fee_receiver: Option<Pubkey>,
    pub new_performance_fee_bps: Option<u16>,
    pub new_withdrawal_fee_bps: Option<u16>,
}

pub fn update_config_handler(ctx: Context<UpdateConfig>, params: UpdateConfigParams) -> Result<()> {
    let config = &mut ctx.accounts.config;

    if let Some(operator) = params.new_operator {
        msg!("Operator updated: {} -> {}", config.operator, operator);
        config.operator = operator;
    }

    if let Some(fee_receiver) = params.new_fee_receiver {
        msg!(
            "Fee receiver updated: {} -> {}",
            config.fee_receiver,
            fee_receiver
        );
        config.fee_receiver = fee_receiver;
    }

    if let Some(performance_fee) = params.new_performance_fee_bps {
        require!(is_valid_fee(performance_fee), VaultError::InvalidFee);
        msg!(
            "Performance fee updated: {} -> {} bps",
            config.performance_fee_bps,
            performance_fee
        );
        config.performance_fee_bps = performance_fee;
    }

    if let Some(withdrawal_fee) = params.new_withdrawal_fee_bps {
        require!(is_valid_fee(withdrawal_fee), VaultError::InvalidFee);
        msg!(
            "Withdrawal fee updated: {} -> {} bps",
            config.withdrawal_fee_bps,
            withdrawal_fee
        );
        config.withdrawal_fee_bps = withdrawal_fee;
    }

    Ok(())
}
