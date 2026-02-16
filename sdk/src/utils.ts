import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import {
  VAULT_PROGRAM_ID,
  VAULT_CONFIG_SEED,
  VAULT_POOL_SEED,
  SHARES_MINT_SEED,
  TOKEN_VAULT_SEED,
  WITHDRAWAL_REQUEST_SEED,
  MAX_FEE_BPS,
} from "./constants";

/**
 * Derive the VaultConfig PDA
 */
export function deriveConfigPda(programId: PublicKey = VAULT_PROGRAM_ID): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_CONFIG_SEED)],
    programId
  );
}

/**
 * Derive the VaultPool PDA for a token mint
 */
export function derivePoolPda(
  tokenMint: PublicKey,
  programId: PublicKey = VAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_POOL_SEED), tokenMint.toBuffer()],
    programId
  );
}

/**
 * Derive the shares mint PDA for a token
 */
export function deriveSharesMintPda(
  tokenMint: PublicKey,
  programId: PublicKey = VAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(SHARES_MINT_SEED), tokenMint.toBuffer()],
    programId
  );
}

/**
 * Derive the token vault PDA for a pool
 */
export function deriveTokenVaultPda(
  tokenMint: PublicKey,
  programId: PublicKey = VAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(TOKEN_VAULT_SEED), tokenMint.toBuffer()],
    programId
  );
}

/**
 * Derive the withdrawal request PDA for a user
 */
export function deriveWithdrawalRequestPda(
  vaultPool: PublicKey,
  user: PublicKey,
  programId: PublicKey = VAULT_PROGRAM_ID
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(WITHDRAWAL_REQUEST_SEED), vaultPool.toBuffer(), user.toBuffer()],
    programId
  );
}

/**
 * Calculate shares to mint for a deposit
 * @param depositAmount Amount of tokens to deposit
 * @param totalAssets Current total assets in pool
 * @param totalShares Current total shares minted
 * @returns Shares to mint
 */
export function calculateSharesToMint(
  depositAmount: BN,
  totalAssets: BN,
  totalShares: BN
): BN {
  if (totalShares.isZero() || totalAssets.isZero()) {
    return depositAmount;
  }
  return depositAmount.mul(totalShares).div(totalAssets);
}

/**
 * Calculate assets to return for a withdrawal
 * @param sharesAmount Amount of shares to burn
 * @param totalAssets Current total assets in pool
 * @param totalShares Current total shares minted
 * @returns Assets to return
 */
export function calculateAssetsToReturn(
  sharesAmount: BN,
  totalAssets: BN,
  totalShares: BN
): BN {
  if (totalShares.isZero()) {
    return new BN(0);
  }
  return sharesAmount.mul(totalAssets).div(totalShares);
}

/**
 * Calculate fee amount
 * @param amount Gross amount
 * @param feeBps Fee in basis points
 * @returns Fee amount
 */
export function calculateFee(amount: BN, feeBps: number): BN {
  return amount.muln(feeBps).divn(MAX_FEE_BPS);
}

/**
 * Calculate net amount after fee
 * @param amount Gross amount
 * @param feeBps Fee in basis points
 * @returns [netAmount, feeAmount]
 */
export function calculateNetAfterFee(amount: BN, feeBps: number): [BN, BN] {
  const fee = calculateFee(amount, feeBps);
  const net = amount.sub(fee);
  return [net, fee];
}

/**
 * Calculate share price (tokens per share)
 * @param totalAssets Total assets in pool
 * @param totalShares Total shares minted
 * @param decimals Token decimals
 * @returns Share price as a number
 */
export function calculateSharePrice(
  totalAssets: BN,
  totalShares: BN,
  decimals: number = 6
): number {
  if (totalShares.isZero()) {
    return 1.0;
  }
  const multiplier = Math.pow(10, decimals);
  return totalAssets.muln(multiplier).div(totalShares).toNumber() / multiplier;
}

/**
 * Format token amount for display
 * @param amount Amount in smallest unit
 * @param decimals Token decimals
 * @returns Formatted string
 */
export function formatTokenAmount(amount: BN, decimals: number = 6): string {
  const divisor = new BN(10).pow(new BN(decimals));
  const whole = amount.div(divisor);
  const fraction = amount.mod(divisor);
  const fractionStr = fraction.toString().padStart(decimals, "0");
  return `${whole.toString()}.${fractionStr}`;
}

/**
 * Parse token amount from string
 * @param amount Amount as string (e.g., "100.5")
 * @param decimals Token decimals
 * @returns Amount in smallest unit
 */
export function parseTokenAmount(amount: string, decimals: number = 6): BN {
  const [whole, fraction = ""] = amount.split(".");
  const paddedFraction = fraction.padEnd(decimals, "0").slice(0, decimals);
  return new BN(whole + paddedFraction);
}

/**
 * Validate fee is within valid range
 * @param feeBps Fee in basis points
 * @returns True if valid
 */
export function isValidFee(feeBps: number): boolean {
  return feeBps >= 0 && feeBps <= MAX_FEE_BPS;
}
