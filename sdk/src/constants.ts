import { PublicKey } from "@solana/web3.js";

/** Program ID - update this after deployment */
export const VAULT_PROGRAM_ID = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);

/** PDA Seeds */
export const VAULT_CONFIG_SEED = "vault_config";
export const VAULT_POOL_SEED = "vault_pool";
export const SHARES_MINT_SEED = "shares_mint";
export const TOKEN_VAULT_SEED = "token_vault";
export const WITHDRAWAL_REQUEST_SEED = "withdrawal_request";

/** Constants */
export const MAX_FEE_BPS = 10000;
export const MIN_DEPOSIT_AMOUNT = 1000;
export const SECONDS_PER_DAY = 86400;
