import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { expect } from "chai";
import { Vault } from "../target/types/vault";

describe("vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Vault as Program<Vault>;

  // Test accounts
  let owner: Keypair;
  let operator: Keypair;
  let user: Keypair;
  let feeReceiver: Keypair;

  // Token mints
  let tokenMint: PublicKey;

  // PDAs
  let configPda: PublicKey;
  let vaultPoolPda: PublicKey;
  let sharesMintPda: PublicKey;
  let tokenVaultPda: PublicKey;

  // Token accounts
  let userTokenAccount: PublicKey;
  let userSharesAccount: PublicKey;
  let feeReceiverTokenAccount: PublicKey;

  before(async () => {
    // Generate keypairs
    owner = Keypair.generate();
    operator = Keypair.generate();
    user = Keypair.generate();
    feeReceiver = Keypair.generate();

    // Airdrop SOL to accounts
    const airdropAmount = 10 * LAMPORTS_PER_SOL;
    await provider.connection.requestAirdrop(owner.publicKey, airdropAmount);
    await provider.connection.requestAirdrop(operator.publicKey, airdropAmount);
    await provider.connection.requestAirdrop(user.publicKey, airdropAmount);
    await provider.connection.requestAirdrop(feeReceiver.publicKey, airdropAmount);

    // Wait for airdrops to confirm
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Create test token mint
    tokenMint = await createMint(
      provider.connection,
      owner,
      owner.publicKey,
      null,
      6 // 6 decimals like USDC
    );

    // Derive PDAs
    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_config")],
      program.programId
    );

    [vaultPoolPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_pool"), tokenMint.toBuffer()],
      program.programId
    );

    [sharesMintPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("shares_mint"), tokenMint.toBuffer()],
      program.programId
    );

    [tokenVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("token_vault"), tokenMint.toBuffer()],
      program.programId
    );

    // Create token accounts for user and fee receiver
    userTokenAccount = await createAccount(
      provider.connection,
      user,
      tokenMint,
      user.publicKey
    );

    feeReceiverTokenAccount = await createAccount(
      provider.connection,
      feeReceiver,
      tokenMint,
      feeReceiver.publicKey
    );

    // Mint tokens to user for testing
    await mintTo(
      provider.connection,
      owner,
      tokenMint,
      userTokenAccount,
      owner,
      1_000_000_000 // 1000 tokens
    );
  });

  describe("Initialize", () => {
    it("should initialize vault config", async () => {
      const performanceFeeBps = 2000; // 20%
      const withdrawalFeeBps = 50; // 0.5%

      await program.methods
        .initialize({
          operator: operator.publicKey,
          feeReceiver: feeReceiver.publicKey,
          performanceFeeBps,
          withdrawalFeeBps,
        })
        .accounts({
          config: configPda,
          owner: owner.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner])
        .rpc();

      const config = await program.account.vaultConfig.fetch(configPda);

      expect(config.owner.toString()).to.equal(owner.publicKey.toString());
      expect(config.operator.toString()).to.equal(operator.publicKey.toString());
      expect(config.performanceFeeBps).to.equal(performanceFeeBps);
      expect(config.withdrawalFeeBps).to.equal(withdrawalFeeBps);
      expect(config.isPaused).to.equal(false);
      expect(config.totalPools.toNumber()).to.equal(0);
    });
  });

  describe("Create Pool", () => {
    it("should create a vault pool for the token", async () => {
      const dailyWithdrawalLimit = new anchor.BN(100_000_000); // 100 tokens

      await program.methods
        .createPool({
          dailyWithdrawalLimit,
        })
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenMint: tokenMint,
          sharesMint: sharesMintPda,
          tokenVault: tokenVaultPda,
          owner: owner.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .signers([owner])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);

      expect(vaultPool.tokenMint.toString()).to.equal(tokenMint.toString());
      expect(vaultPool.sharesMint.toString()).to.equal(sharesMintPda.toString());
      expect(vaultPool.isActive).to.equal(true);
      expect(vaultPool.totalAssets.toNumber()).to.equal(0);
      expect(vaultPool.totalShares.toNumber()).to.equal(0);

      // Create user's shares account after shares mint is created
      userSharesAccount = await createAccount(
        provider.connection,
        user,
        sharesMintPda,
        user.publicKey
      );
    });
  });

  describe("Deposit", () => {
    it("should deposit tokens and receive shares", async () => {
      const depositAmount = new anchor.BN(100_000_000); // 100 tokens

      const userTokenBefore = await getAccount(
        provider.connection,
        userTokenAccount
      );

      await program.methods
        .deposit(depositAmount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenVault: tokenVaultPda,
          sharesMint: sharesMintPda,
          userTokenAccount: userTokenAccount,
          userSharesAccount: userSharesAccount,
          user: user.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);
      const userTokenAfter = await getAccount(
        provider.connection,
        userTokenAccount
      );
      const userSharesAfter = await getAccount(
        provider.connection,
        userSharesAccount
      );

      // First deposit: 1:1 ratio
      expect(vaultPool.totalAssets.toNumber()).to.equal(depositAmount.toNumber());
      expect(vaultPool.totalShares.toNumber()).to.equal(depositAmount.toNumber());
      expect(Number(userSharesAfter.amount)).to.equal(depositAmount.toNumber());
      expect(Number(userTokenBefore.amount) - Number(userTokenAfter.amount)).to.equal(
        depositAmount.toNumber()
      );
    });
  });

  describe("Withdraw", () => {
    it("should withdraw tokens by burning shares", async () => {
      const sharesAmount = new anchor.BN(50_000_000); // 50 shares

      const userSharesBefore = await getAccount(
        provider.connection,
        userSharesAccount
      );

      await program.methods
        .withdraw(sharesAmount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenVault: tokenVaultPda,
          sharesMint: sharesMintPda,
          userTokenAccount: userTokenAccount,
          userSharesAccount: userSharesAccount,
          feeReceiverAccount: feeReceiverTokenAccount,
          feeReceiver: feeReceiver.publicKey,
          user: user.publicKey,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);
      const userSharesAfter = await getAccount(
        provider.connection,
        userSharesAccount
      );

      expect(Number(userSharesBefore.amount) - Number(userSharesAfter.amount)).to.equal(
        sharesAmount.toNumber()
      );
      expect(vaultPool.totalShares.toNumber()).to.equal(50_000_000);
    });
  });

  describe("Pause/Unpause", () => {
    it("should pause the vault", async () => {
      await program.methods
        .pause()
        .accounts({
          config: configPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const config = await program.account.vaultConfig.fetch(configPda);
      expect(config.isPaused).to.equal(true);
    });

    it("should unpause the vault", async () => {
      await program.methods
        .unpause()
        .accounts({
          config: configPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const config = await program.account.vaultConfig.fetch(configPda);
      expect(config.isPaused).to.equal(false);
    });
  });

  describe("Inject Yield", () => {
    it("should inject yield and auto-compound", async () => {
      // Create operator's token account and mint tokens for yield injection
      const operatorTokenAccount = await createAccount(
        provider.connection,
        operator,
        tokenMint,
        operator.publicKey
      );

      await mintTo(
        provider.connection,
        owner,
        tokenMint,
        operatorTokenAccount,
        owner,
        10_000_000 // 10 tokens yield
      );

      const vaultPoolBefore = await program.account.vaultPool.fetch(vaultPoolPda);
      const yieldAmount = new anchor.BN(10_000_000);

      await program.methods
        .injectYield(yieldAmount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenVault: tokenVaultPda,
          yieldSource: operatorTokenAccount,
          feeReceiverAccount: feeReceiverTokenAccount,
          feeReceiver: feeReceiver.publicKey,
          authority: operator.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([operator])
        .rpc();

      const vaultPoolAfter = await program.account.vaultPool.fetch(vaultPoolPda);

      // 20% performance fee, so net yield = 8 tokens
      const expectedNetYield = 8_000_000;
      expect(
        vaultPoolAfter.totalAssets.toNumber() - vaultPoolBefore.totalAssets.toNumber()
      ).to.equal(expectedNetYield);
    });
  });

  describe("Update Config", () => {
    it("should update performance fee", async () => {
      const newPerformanceFee = 1500; // 15%

      await program.methods
        .updateConfig({
          newOperator: null,
          newFeeReceiver: null,
          newPerformanceFeeBps: newPerformanceFee,
          newWithdrawalFeeBps: null,
        })
        .accounts({
          config: configPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const config = await program.account.vaultConfig.fetch(configPda);
      expect(config.performanceFeeBps).to.equal(newPerformanceFee);
    });

    it("should update operator", async () => {
      const newOperator = Keypair.generate();

      await program.methods
        .updateConfig({
          newOperator: newOperator.publicKey,
          newFeeReceiver: null,
          newPerformanceFeeBps: null,
          newWithdrawalFeeBps: null,
        })
        .accounts({
          config: configPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const config = await program.account.vaultConfig.fetch(configPda);
      expect(config.operator.toString()).to.equal(newOperator.publicKey.toString());

      // Restore original operator for subsequent tests
      await program.methods
        .updateConfig({
          newOperator: operator.publicKey,
          newFeeReceiver: null,
          newPerformanceFeeBps: null,
          newWithdrawalFeeBps: null,
        })
        .accounts({
          config: configPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();
    });
  });

  describe("Set Pool Status", () => {
    it("should deactivate pool", async () => {
      await program.methods
        .setPoolStatus(false)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);
      expect(vaultPool.isActive).to.equal(false);
    });

    it("should reject deposit when pool is inactive", async () => {
      const depositAmount = new anchor.BN(10_000_000);

      try {
        await program.methods
          .deposit(depositAmount)
          .accounts({
            config: configPda,
            vaultPool: vaultPoolPda,
            tokenVault: tokenVaultPda,
            sharesMint: sharesMintPda,
            userTokenAccount: userTokenAccount,
            userSharesAccount: userSharesAccount,
            user: user.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("PoolNotActive");
      }
    });

    it("should reactivate pool", async () => {
      await program.methods
        .setPoolStatus(true)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);
      expect(vaultPool.isActive).to.equal(true);
    });
  });

  describe("Set Withdrawal Limit", () => {
    it("should update daily withdrawal limit", async () => {
      const newLimit = new anchor.BN(200_000_000); // 200 tokens

      await program.methods
        .setWithdrawalLimit(newLimit)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          owner: owner.publicKey,
        })
        .signers([owner])
        .rpc();

      const vaultPool = await program.account.vaultPool.fetch(vaultPoolPda);
      expect(vaultPool.dailyWithdrawalLimit.toNumber()).to.equal(newLimit.toNumber());
    });
  });

  describe("Withdrawal Request Queue", () => {
    let withdrawalRequestPda: PublicKey;

    it("should create withdrawal request", async () => {
      const sharesAmount = new anchor.BN(10_000_000); // 10 shares

      [withdrawalRequestPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("withdrawal_request"),
          vaultPoolPda.toBuffer(),
          user.publicKey.toBuffer(),
        ],
        program.programId
      );

      await program.methods
        .requestWithdrawal(sharesAmount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          withdrawalRequest: withdrawalRequestPda,
          user: user.publicKey,
          clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      const request = await program.account.withdrawalRequest.fetch(withdrawalRequestPda);
      expect(request.user.toString()).to.equal(user.publicKey.toString());
      expect(request.sharesAmount.toNumber()).to.equal(sharesAmount.toNumber());
      expect(request.isProcessed).to.equal(false);
    });

    it("should cancel withdrawal request", async () => {
      await program.methods
        .cancelWithdrawal()
        .accounts({
          withdrawalRequest: withdrawalRequestPda,
          user: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Account should be closed
      try {
        await program.account.withdrawalRequest.fetch(withdrawalRequestPda);
        expect.fail("Account should be closed");
      } catch (err: any) {
        expect(err.message).to.include("Account does not exist");
      }
    });
  });

  describe("Strategy Fund Management", () => {
    let operatorTokenAccount: PublicKey;

    before(async () => {
      // Create operator's token account if not exists
      operatorTokenAccount = await createAccount(
        provider.connection,
        operator,
        tokenMint,
        operator.publicKey
      );
    });

    it("should withdraw funds for strategy deployment", async () => {
      const amount = new anchor.BN(10_000_000); // 10 tokens
      const vaultBefore = await getAccount(provider.connection, tokenVaultPda);

      await program.methods
        .withdrawForStrategy(amount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenVault: tokenVaultPda,
          strategyDestination: operatorTokenAccount,
          authority: operator.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([operator])
        .rpc();

      const vaultAfter = await getAccount(provider.connection, tokenVaultPda);
      expect(Number(vaultBefore.amount) - Number(vaultAfter.amount)).to.equal(
        amount.toNumber()
      );
    });

    it("should return funds from strategy", async () => {
      const amount = new anchor.BN(10_000_000); // 10 tokens
      const vaultBefore = await getAccount(provider.connection, tokenVaultPda);

      await program.methods
        .returnFromStrategy(amount)
        .accounts({
          config: configPda,
          vaultPool: vaultPoolPda,
          tokenVault: tokenVaultPda,
          strategySource: operatorTokenAccount,
          authority: operator.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([operator])
        .rpc();

      const vaultAfter = await getAccount(provider.connection, tokenVaultPda);
      expect(Number(vaultAfter.amount) - Number(vaultBefore.amount)).to.equal(
        amount.toNumber()
      );
    });
  });

  describe("Access Control", () => {
    it("should reject non-owner from pausing", async () => {
      try {
        await program.methods
          .pause()
          .accounts({
            config: configPda,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("UnauthorizedOwner");
      }
    });

    it("should reject non-authorized from injecting yield", async () => {
      const unauthorizedUser = Keypair.generate();
      await provider.connection.requestAirdrop(
        unauthorizedUser.publicKey,
        LAMPORTS_PER_SOL
      );
      await new Promise((resolve) => setTimeout(resolve, 500));

      const unauthorizedTokenAccount = await createAccount(
        provider.connection,
        unauthorizedUser,
        tokenMint,
        unauthorizedUser.publicKey
      );

      try {
        await program.methods
          .injectYield(new anchor.BN(1_000_000))
          .accounts({
            config: configPda,
            vaultPool: vaultPoolPda,
            tokenVault: tokenVaultPda,
            yieldSource: unauthorizedTokenAccount,
            feeReceiverAccount: feeReceiverTokenAccount,
            feeReceiver: feeReceiver.publicKey,
            authority: unauthorizedUser.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([unauthorizedUser])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("Unauthorized");
      }
    });
  });

  describe("Edge Cases", () => {
    it("should reject zero deposit", async () => {
      try {
        await program.methods
          .deposit(new anchor.BN(0))
          .accounts({
            config: configPda,
            vaultPool: vaultPoolPda,
            tokenVault: tokenVaultPda,
            sharesMint: sharesMintPda,
            userTokenAccount: userTokenAccount,
            userSharesAccount: userSharesAccount,
            user: user.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("ZeroDeposit");
      }
    });

    it("should reject deposit below minimum", async () => {
      try {
        await program.methods
          .deposit(new anchor.BN(100)) // Below MIN_DEPOSIT_AMOUNT (1000)
          .accounts({
            config: configPda,
            vaultPool: vaultPoolPda,
            tokenVault: tokenVaultPda,
            sharesMint: sharesMintPda,
            userTokenAccount: userTokenAccount,
            userSharesAccount: userSharesAccount,
            user: user.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("MinimumDepositNotMet");
      }
    });

    it("should reject zero withdrawal", async () => {
      try {
        await program.methods
          .withdraw(new anchor.BN(0))
          .accounts({
            config: configPda,
            vaultPool: vaultPoolPda,
            tokenVault: tokenVaultPda,
            sharesMint: sharesMintPda,
            userTokenAccount: userTokenAccount,
            userSharesAccount: userSharesAccount,
            feeReceiverAccount: feeReceiverTokenAccount,
            feeReceiver: feeReceiver.publicKey,
            user: user.publicKey,
            clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("ZeroWithdrawal");
      }
    });

    it("should reject invalid fee update (> 100%)", async () => {
      try {
        await program.methods
          .updateConfig({
            newOperator: null,
            newFeeReceiver: null,
            newPerformanceFeeBps: 15000, // 150% - invalid
            newWithdrawalFeeBps: null,
          })
          .accounts({
            config: configPda,
            owner: owner.publicKey,
          })
          .signers([owner])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (err: any) {
        expect(err.error.errorCode.code).to.equal("InvalidFee");
      }
    });
  });
});
