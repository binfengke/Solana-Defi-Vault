use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod vault {
    use super::*;

    // ============ Admin Instructions (Owner Only) ============

    /// Initialize the global vault configuration
    pub fn initialize(ctx: Context<Initialize>, params: InitializeParams) -> Result<()> {
        admin::initialize::initialize_handler(ctx, params)
    }

    /// Update vault configuration parameters
    pub fn update_config(ctx: Context<UpdateConfig>, params: UpdateConfigParams) -> Result<()> {
        admin::update_config::update_config_handler(ctx, params)
    }

    /// Create a new vault pool for a token
    pub fn create_pool(ctx: Context<CreatePool>, params: CreatePoolParams) -> Result<()> {
        admin::create_pool::create_pool_handler(ctx, params)
    }

    /// Pause all vault operations
    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        admin::emergency::pause_handler(ctx)
    }

    /// Unpause vault operations
    pub fn unpause(ctx: Context<Pause>) -> Result<()> {
        admin::emergency::unpause_handler(ctx)
    }

    /// Set pool active status
    pub fn set_pool_status(ctx: Context<SetPoolStatus>, is_active: bool) -> Result<()> {
        admin::emergency::set_pool_active_handler(ctx, is_active)
    }

    /// Set daily withdrawal limit for a pool
    pub fn set_withdrawal_limit(
        ctx: Context<SetWithdrawalLimit>,
        daily_limit: u64,
    ) -> Result<()> {
        admin::emergency::set_withdrawal_limit_handler(ctx, daily_limit)
    }

    /// Emergency withdraw all funds from a pool
    pub fn emergency_withdraw(ctx: Context<EmergencyWithdraw>) -> Result<()> {
        admin::emergency::emergency_withdraw_handler(ctx)
    }

    /// Transfer ownership to a new owner
    pub fn transfer_ownership(ctx: Context<TransferOwnership>) -> Result<()> {
        admin::emergency::transfer_ownership_handler(ctx)
    }

    /// Close a processed withdrawal request (reclaim rent)
    pub fn close_withdrawal_request(ctx: Context<CloseWithdrawalRequest>) -> Result<()> {
        admin::close::close_withdrawal_request_handler(ctx)
    }

    /// Close an empty and inactive vault pool
    pub fn close_pool(ctx: Context<ClosePool>) -> Result<()> {
        admin::close::close_pool_handler(ctx)
    }

    // ============ User Instructions ============

    /// Deposit tokens into the vault and receive shares
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        user::deposit::deposit_handler(ctx, amount)
    }

    /// Withdraw tokens by burning shares (instant if liquidity available)
    pub fn withdraw(ctx: Context<Withdraw>, shares_amount: u64) -> Result<()> {
        user::withdraw::withdraw_handler(ctx, shares_amount)
    }

    /// Request a withdrawal (enters queue if liquidity insufficient)
    pub fn request_withdrawal(
        ctx: Context<RequestWithdrawal>,
        shares_amount: u64,
    ) -> Result<()> {
        user::withdraw::request_withdrawal_handler(ctx, shares_amount)
    }

    /// Cancel a pending withdrawal request
    pub fn cancel_withdrawal(ctx: Context<CancelWithdrawal>) -> Result<()> {
        user::withdraw::cancel_withdrawal_handler(ctx)
    }

    // ============ Operator Instructions ============

    /// Inject yield into the vault (auto-compounds)
    pub fn inject_yield(ctx: Context<InjectYield>, yield_amount: u64) -> Result<()> {
        operator::inject_yield::inject_yield_handler(ctx, yield_amount)
    }

    /// Withdraw funds for strategy deployment
    pub fn withdraw_for_strategy(
        ctx: Context<WithdrawForStrategy>,
        amount: u64,
    ) -> Result<()> {
        operator::inject_yield::withdraw_for_strategy_handler(ctx, amount)
    }

    /// Return funds from strategy
    pub fn return_from_strategy(
        ctx: Context<ReturnFromStrategy>,
        amount: u64,
    ) -> Result<()> {
        operator::inject_yield::return_from_strategy_handler(ctx, amount)
    }

    /// Process a pending withdrawal request from the queue
    pub fn process_withdrawal_queue(ctx: Context<ProcessWithdrawalQueue>) -> Result<()> {
        operator::process_queue::process_withdrawal_queue_handler(ctx)
    }

    // ============ View Functions ============

    /// Preview a deposit to see expected shares
    pub fn preview_deposit(ctx: Context<PreviewDeposit>, amount: u64) -> Result<DepositPreview> {
        views::preview_deposit_handler(ctx, amount)
    }

    /// Preview a withdrawal to see expected assets
    pub fn preview_withdraw(
        ctx: Context<PreviewWithdraw>,
        shares_amount: u64,
    ) -> Result<WithdrawPreview> {
        views::preview_withdraw_handler(ctx, shares_amount)
    }

    /// Get pool information
    pub fn get_pool_info(ctx: Context<PreviewDeposit>) -> Result<PoolInfoView> {
        views::get_pool_info_handler(ctx)
    }
}
