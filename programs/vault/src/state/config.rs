use anchor_lang::prelude::*;

/// Global configuration for the Vault protocol - Singleton PDA
#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    /// Owner has highest privilege (add tokens, change admin, emergency actions)
    pub owner: Pubkey,

    /// Operator handles daily operations (inject yield, process withdrawals)
    pub operator: Pubkey,

    /// Global pause switch - when true, all deposits/withdrawals are disabled
    pub is_paused: bool,

    /// Performance fee in basis points (e.g., 2000 = 20%)
    pub performance_fee_bps: u16,

    /// Withdrawal fee in basis points (e.g., 50 = 0.5%)
    pub withdrawal_fee_bps: u16,

    /// Address that receives collected fees
    pub fee_receiver: Pubkey,

    /// Total number of vault pools created
    pub total_pools: u64,

    /// PDA bump seed
    pub bump: u8,
}

impl VaultConfig {
    pub const SEED_PREFIX: &'static [u8] = b"vault_config";

    /// Check if the signer is the owner
    pub fn is_owner(&self, signer: &Pubkey) -> bool {
        self.owner == *signer
    }

    /// Check if the signer is the operator
    pub fn is_operator(&self, signer: &Pubkey) -> bool {
        self.operator == *signer
    }

    /// Check if the signer is either owner or operator
    pub fn is_authorized(&self, signer: &Pubkey) -> bool {
        self.is_owner(signer) || self.is_operator(signer)
    }
}
