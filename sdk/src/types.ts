import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

/** Vault Configuration account data */
export interface VaultConfig {
  owner: PublicKey;
  operator: PublicKey;
  isPaused: boolean;
  performanceFeeBps: number;
  withdrawalFeeBps: number;
  feeReceiver: PublicKey;
  totalPools: BN;
  bump: number;
}

/** Vault Pool account data */
export interface VaultPool {
  config: PublicKey;
  tokenMint: PublicKey;
  sharesMint: PublicKey;
  tokenVault: PublicKey;
  totalAssets: BN;
  totalShares: BN;
  dailyWithdrawalLimit: BN;
  withdrawnToday: BN;
  lastWithdrawalDay: BN;
  isActive: boolean;
  poolIndex: BN;
  bump: number;
  sharesMintBump: number;
}

/** Withdrawal Request account data */
export interface WithdrawalRequest {
  user: PublicKey;
  vaultPool: PublicKey;
  sharesAmount: BN;
  requestedAt: BN;
  isProcessed: boolean;
  bump: number;
}

/** Initialize parameters */
export interface InitializeParams {
  operator: PublicKey;
  feeReceiver: PublicKey;
  performanceFeeBps: number;
  withdrawalFeeBps: number;
}

/** Create pool parameters */
export interface CreatePoolParams {
  dailyWithdrawalLimit: BN;
}

/** Update config parameters */
export interface UpdateConfigParams {
  newOperator: PublicKey | null;
  newFeeReceiver: PublicKey | null;
  newPerformanceFeeBps: number | null;
  newWithdrawalFeeBps: number | null;
}

/** Pool info with calculated values */
export interface PoolInfo {
  pool: VaultPool;
  sharePrice: number;
  tvl: BN;
  availableLiquidity: BN;
}

/** User position in a pool */
export interface UserPosition {
  sharesBalance: BN;
  estimatedValue: BN;
  sharePrice: number;
}
