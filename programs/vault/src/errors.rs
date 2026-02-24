use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    // === Access Control Errors ===
    #[msg("Only the owner can perform this action")]
    UnauthorizedOwner,

    #[msg("Only the operator can perform this action")]
    UnauthorizedOperator,

    #[msg("Unauthorized: requires owner or operator")]
    Unauthorized,

    // === State Errors ===
    #[msg("Vault is currently paused")]
    VaultPaused,

    #[msg("Pool is not active")]
    PoolNotActive,

    #[msg("Pool is already active")]
    PoolAlreadyActive,

    // === Deposit/Withdraw Errors ===
    #[msg("Deposit amount must be greater than zero")]
    ZeroDeposit,

    #[msg("Withdrawal amount must be greater than zero")]
    ZeroWithdrawal,

    #[msg("Insufficient shares balance")]
    InsufficientShares,

    #[msg("Insufficient vault liquidity")]
    InsufficientLiquidity,

    #[msg("Daily withdrawal limit exceeded")]
    DailyLimitExceeded,

    #[msg("Minimum deposit amount not met")]
    MinimumDepositNotMet,

    // === Calculation Errors ===
    #[msg("Math overflow occurred")]
    MathOverflow,

    #[msg("Invalid shares calculation")]
    InvalidSharesCalculation,

    #[msg("Division by zero")]
    DivisionByZero,

    // === Withdrawal Queue Errors ===
    #[msg("Withdrawal request already exists")]
    WithdrawalRequestExists,

    #[msg("Withdrawal request not found")]
    WithdrawalRequestNotFound,

    #[msg("Withdrawal request already processed")]
    WithdrawalAlreadyProcessed,

    #[msg("User does not match withdrawal request")]
    UserMismatch,

    // === Configuration Errors ===
    #[msg("Invalid fee: must be <= 10000 basis points")]
    InvalidFee,

    #[msg("Invalid token mint")]
    InvalidTokenMint,

    #[msg("Invalid fee receiver token account")]
    InvalidFeeReceiverAccount,

    #[msg("Token already whitelisted")]
    TokenAlreadyWhitelisted,
}
