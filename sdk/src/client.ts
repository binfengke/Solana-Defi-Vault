import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  Keypair,
  Commitment,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  getAccount,
} from "@solana/spl-token";
import { Program, AnchorProvider, BN, Idl } from "@coral-xyz/anchor";
import {
  VaultConfig,
  VaultPool,
  WithdrawalRequest,
  InitializeParams,
  CreatePoolParams,
  UpdateConfigParams,
  PoolInfo,
  UserPosition,
} from "./types";
import {
  deriveConfigPda,
  derivePoolPda,
  deriveSharesMintPda,
  deriveTokenVaultPda,
  deriveWithdrawalRequestPda,
  calculateSharesToMint,
  calculateAssetsToReturn,
  calculateSharePrice,
  calculateNetAfterFee,
} from "./utils";
import { VAULT_PROGRAM_ID } from "./constants";

/**
 * VaultClient - Main client for interacting with the Vault program
 */
export class VaultClient {
  readonly connection: Connection;
  readonly programId: PublicKey;
  readonly program: Program;
  readonly configPda: PublicKey;

  constructor(
    connection: Connection,
    programId: PublicKey = VAULT_PROGRAM_ID,
    idl?: Idl
  ) {
    this.connection = connection;
    this.programId = programId;
    [this.configPda] = deriveConfigPda(programId);

    // Create a read-only provider for fetching accounts
    const readOnlyProvider = new AnchorProvider(
      connection,
      {
        publicKey: PublicKey.default,
        signTransaction: async (tx) => tx,
        signAllTransactions: async (txs) => txs,
      },
      { commitment: "confirmed" }
    );

    // Initialize program with IDL if provided
    if (idl) {
      this.program = new Program(idl, programId, readOnlyProvider);
    } else {
      // For read-only operations, we'll fetch accounts directly
      this.program = null as any;
    }
  }

  // ============ Read Methods ============

  /**
   * Fetch the global vault configuration
   */
  async getConfig(): Promise<VaultConfig | null> {
    try {
      const account = await this.program?.account.vaultConfig.fetch(this.configPda);
      return account as VaultConfig;
    } catch {
      return null;
    }
  }

  /**
   * Fetch a vault pool by token mint
   */
  async getPool(tokenMint: PublicKey): Promise<VaultPool | null> {
    try {
      const [poolPda] = derivePoolPda(tokenMint, this.programId);
      const account = await this.program?.account.vaultPool.fetch(poolPda);
      return account as VaultPool;
    } catch {
      return null;
    }
  }

  /**
   * Get all vault pools
   */
  async getAllPools(): Promise<VaultPool[]> {
    try {
      const accounts = await this.program?.account.vaultPool.all();
      return accounts?.map((a) => a.account as VaultPool) || [];
    } catch {
      return [];
    }
  }

  /**
   * Get pool info with calculated values
   */
  async getPoolInfo(tokenMint: PublicKey, decimals: number = 6): Promise<PoolInfo | null> {
    const pool = await this.getPool(tokenMint);
    if (!pool) return null;

    const [tokenVaultPda] = deriveTokenVaultPda(tokenMint, this.programId);
    let availableLiquidity = new BN(0);

    try {
      const vaultAccount = await getAccount(this.connection, tokenVaultPda);
      availableLiquidity = new BN(vaultAccount.amount.toString());
    } catch {
      // Vault account doesn't exist or error
    }

    return {
      pool,
      sharePrice: calculateSharePrice(pool.totalAssets, pool.totalShares, decimals),
      tvl: pool.totalAssets,
      availableLiquidity,
    };
  }

  /**
   * Get user's position in a pool
   */
  async getUserPosition(
    tokenMint: PublicKey,
    user: PublicKey,
    decimals: number = 6
  ): Promise<UserPosition | null> {
    const pool = await this.getPool(tokenMint);
    if (!pool) return null;

    const [sharesMintPda] = deriveSharesMintPda(tokenMint, this.programId);
    const userSharesAta = await getAssociatedTokenAddress(sharesMintPda, user);

    let sharesBalance = new BN(0);
    try {
      const sharesAccount = await getAccount(this.connection, userSharesAta);
      sharesBalance = new BN(sharesAccount.amount.toString());
    } catch {
      // User doesn't have shares account
    }

    const estimatedValue = calculateAssetsToReturn(
      sharesBalance,
      pool.totalAssets,
      pool.totalShares
    );

    return {
      sharesBalance,
      estimatedValue,
      sharePrice: calculateSharePrice(pool.totalAssets, pool.totalShares, decimals),
    };
  }

  /**
   * Get user's pending withdrawal request
   */
  async getWithdrawalRequest(
    tokenMint: PublicKey,
    user: PublicKey
  ): Promise<WithdrawalRequest | null> {
    try {
      const [poolPda] = derivePoolPda(tokenMint, this.programId);
      const [requestPda] = deriveWithdrawalRequestPda(poolPda, user, this.programId);
      const account = await this.program?.account.withdrawalRequest.fetch(requestPda);
      return account as WithdrawalRequest;
    } catch {
      return null;
    }
  }

  // ============ PDA Helpers ============

  /**
   * Get all PDAs for a token mint
   */
  getPoolPdas(tokenMint: PublicKey) {
    const [poolPda, poolBump] = derivePoolPda(tokenMint, this.programId);
    const [sharesMintPda, sharesMintBump] = deriveSharesMintPda(tokenMint, this.programId);
    const [tokenVaultPda, tokenVaultBump] = deriveTokenVaultPda(tokenMint, this.programId);

    return {
      poolPda,
      poolBump,
      sharesMintPda,
      sharesMintBump,
      tokenVaultPda,
      tokenVaultBump,
    };
  }

  // ============ Simulation Helpers ============

  /**
   * Simulate a deposit to get expected shares
   */
  async simulateDeposit(
    tokenMint: PublicKey,
    depositAmount: BN
  ): Promise<{ sharesToReceive: BN; sharePrice: number } | null> {
    const pool = await this.getPool(tokenMint);
    if (!pool) return null;

    const sharesToReceive = calculateSharesToMint(
      depositAmount,
      pool.totalAssets,
      pool.totalShares
    );

    return {
      sharesToReceive,
      sharePrice: calculateSharePrice(pool.totalAssets, pool.totalShares),
    };
  }

  /**
   * Simulate a withdrawal to get expected assets
   */
  async simulateWithdrawal(
    tokenMint: PublicKey,
    sharesAmount: BN
  ): Promise<{
    grossAssets: BN;
    netAssets: BN;
    fee: BN;
  } | null> {
    const [pool, config] = await Promise.all([
      this.getPool(tokenMint),
      this.getConfig(),
    ]);

    if (!pool || !config) return null;

    const grossAssets = calculateAssetsToReturn(
      sharesAmount,
      pool.totalAssets,
      pool.totalShares
    );

    const [netAssets, fee] = calculateNetAfterFee(grossAssets, config.withdrawalFeeBps);

    return { grossAssets, netAssets, fee };
  }

  // ============ Instruction Builders ============

  /**
   * Build initialize instruction
   */
  buildInitializeInstruction(
    owner: PublicKey,
    params: InitializeParams
  ): TransactionInstruction {
    return this.program.methods
      .initialize(params)
      .accounts({
        config: this.configPda,
        owner,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  /**
   * Build create pool instruction
   */
  async buildCreatePoolInstruction(
    owner: PublicKey,
    tokenMint: PublicKey,
    params: CreatePoolParams
  ): Promise<TransactionInstruction> {
    const { poolPda, sharesMintPda, tokenVaultPda } = this.getPoolPdas(tokenMint);

    return this.program.methods
      .createPool(params)
      .accounts({
        config: this.configPda,
        vaultPool: poolPda,
        tokenMint,
        sharesMint: sharesMintPda,
        tokenVault: tokenVaultPda,
        owner,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  /**
   * Build deposit instruction
   */
  async buildDepositInstruction(
    user: PublicKey,
    tokenMint: PublicKey,
    amount: BN
  ): Promise<TransactionInstruction[]> {
    const { poolPda, sharesMintPda, tokenVaultPda } = this.getPoolPdas(tokenMint);

    const userTokenAta = await getAssociatedTokenAddress(tokenMint, user);
    const userSharesAta = await getAssociatedTokenAddress(sharesMintPda, user);

    const instructions: TransactionInstruction[] = [];

    // Check if user needs shares ATA
    try {
      await getAccount(this.connection, userSharesAta);
    } catch {
      instructions.push(
        createAssociatedTokenAccountInstruction(user, userSharesAta, user, sharesMintPda)
      );
    }

    instructions.push(
      await this.program.methods
        .deposit(amount)
        .accounts({
          config: this.configPda,
          vaultPool: poolPda,
          tokenVault: tokenVaultPda,
          sharesMint: sharesMintPda,
          userTokenAccount: userTokenAta,
          userSharesAccount: userSharesAta,
          user,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .instruction()
    );

    return instructions;
  }

  /**
   * Build withdraw instruction
   */
  async buildWithdrawInstruction(
    user: PublicKey,
    tokenMint: PublicKey,
    sharesAmount: BN
  ): Promise<TransactionInstruction> {
    const config = await this.getConfig();
    if (!config) throw new Error("Config not found");

    const { poolPda, sharesMintPda, tokenVaultPda } = this.getPoolPdas(tokenMint);

    const userTokenAta = await getAssociatedTokenAddress(tokenMint, user);
    const userSharesAta = await getAssociatedTokenAddress(sharesMintPda, user);
    const feeReceiverAta = await getAssociatedTokenAddress(tokenMint, config.feeReceiver);

    return this.program.methods
      .withdraw(sharesAmount)
      .accounts({
        config: this.configPda,
        vaultPool: poolPda,
        tokenVault: tokenVaultPda,
        sharesMint: sharesMintPda,
        userTokenAccount: userTokenAta,
        userSharesAccount: userSharesAta,
        feeReceiverAccount: feeReceiverAta,
        feeReceiver: config.feeReceiver,
        user,
        clock: SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
  }

  /**
   * Build request withdrawal instruction
   */
  async buildRequestWithdrawalInstruction(
    user: PublicKey,
    tokenMint: PublicKey,
    sharesAmount: BN
  ): Promise<TransactionInstruction> {
    const [poolPda] = derivePoolPda(tokenMint, this.programId);
    const [requestPda] = deriveWithdrawalRequestPda(poolPda, user, this.programId);

    return this.program.methods
      .requestWithdrawal(sharesAmount)
      .accounts({
        config: this.configPda,
        vaultPool: poolPda,
        withdrawalRequest: requestPda,
        user,
        clock: SYSVAR_CLOCK_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
  }

  /**
   * Build cancel withdrawal instruction
   */
  async buildCancelWithdrawalInstruction(
    user: PublicKey,
    tokenMint: PublicKey
  ): Promise<TransactionInstruction> {
    const [poolPda] = derivePoolPda(tokenMint, this.programId);
    const [requestPda] = deriveWithdrawalRequestPda(poolPda, user, this.programId);

    return this.program.methods
      .cancelWithdrawal()
      .accounts({
        withdrawalRequest: requestPda,
        user,
      })
      .instruction();
  }

  /**
   * Build pause instruction
   */
  async buildPauseInstruction(owner: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .pause()
      .accounts({
        config: this.configPda,
        owner,
      })
      .instruction();
  }

  /**
   * Build unpause instruction
   */
  async buildUnpauseInstruction(owner: PublicKey): Promise<TransactionInstruction> {
    return this.program.methods
      .unpause()
      .accounts({
        config: this.configPda,
        owner,
      })
      .instruction();
  }

  /**
   * Build update config instruction
   */
  async buildUpdateConfigInstruction(
    owner: PublicKey,
    params: UpdateConfigParams
  ): Promise<TransactionInstruction> {
    return this.program.methods
      .updateConfig(params)
      .accounts({
        config: this.configPda,
        owner,
      })
      .instruction();
  }

  /**
   * Build inject yield instruction
   */
  async buildInjectYieldInstruction(
    authority: PublicKey,
    tokenMint: PublicKey,
    yieldAmount: BN,
    yieldSourceAta: PublicKey
  ): Promise<TransactionInstruction> {
    const config = await this.getConfig();
    if (!config) throw new Error("Config not found");

    const { poolPda, tokenVaultPda } = this.getPoolPdas(tokenMint);
    const feeReceiverAta = await getAssociatedTokenAddress(tokenMint, config.feeReceiver);

    return this.program.methods
      .injectYield(yieldAmount)
      .accounts({
        config: this.configPda,
        vaultPool: poolPda,
        tokenVault: tokenVaultPda,
        yieldSource: yieldSourceAta,
        feeReceiverAccount: feeReceiverAta,
        feeReceiver: config.feeReceiver,
        authority,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();
  }
}
