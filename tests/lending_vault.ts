import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LendingVault } from "../target/types/lending_vault";
import { CollateralManager } from "../target/types/collateral_manager";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { expect } from "chai";
import { createTestMint, createFundedAta, getTokenBalance } from "./helpers";

describe("lending-vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program   = anchor.workspace.LendingVault as Program<LendingVault>;
  const cmProgram = anchor.workspace.CollateralManager as Program<CollateralManager>;
  const wallet    = provider.wallet as anchor.Wallet;

  // Shared across all describe blocks
  let usdcMint:           PublicKey;
  let lendingVaultPda:    PublicKey;
  let vaultUsdcAta:       PublicKey;
  let depositorAta:       PublicKey;
  let borrowerAta:        PublicKey;
  let lpMint:             PublicKey;
  let ownerLpAta:         PublicKey;
  let collateralVaultPda: PublicKey;
  let custodyAtaPda:      PublicKey;
  let loanPda:            PublicKey;

  before(async () => {
    usdcMint = await createTestMint(provider, 6);
    [lendingVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lending_vault")], program.programId,
    );
    [vaultUsdcAta] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_usdc"), lendingVaultPda.toBuffer()], program.programId,
    );
  });

  describe("initialize_vault", () => {
    it("initialises vault with 0.1% fee and zero liquidity", async () => {
      await program.methods.initializeVault()
        .accounts({
          authority: wallet.publicKey,
          lendingVault: lendingVaultPda,
          usdcMint,
          vaultUsdcAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const v = await program.account.lendingVault.fetch(lendingVaultPda);
      expect(v.borrowFeeBps).to.eq(10);
      expect(v.totalLiquidity.toNumber()).to.eq(0);
      expect(v.availableLiquidity.toNumber()).to.eq(0);
    });
  });

  describe("deposit_vault_liquidity + issue_loan", () => {
    before(async () => {
      // Single wallet ATA for USDC; wallet plays both depositor and borrower roles.
      depositorAta = await createFundedAta(provider, usdcMint, wallet.publicKey, 1_000_000_000n); // $1000
      borrowerAta  = depositorAta;

      // Collateral vault valued at $200 (max borrow = $100)
      lpMint     = await createTestMint(provider);
      ownerLpAta = await createFundedAta(provider, lpMint, wallet.publicKey, 1n);
      [collateralVaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("collateral_vault"), wallet.publicKey.toBuffer(), lpMint.toBuffer()],
        cmProgram.programId,
      );
      [custodyAtaPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_custody"), collateralVaultPda.toBuffer()], cmProgram.programId,
      );
      [loanPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("loan"), collateralVaultPda.toBuffer()], program.programId,
      );

      await cmProgram.methods.depositCollateral({ meteora: {} })
        .accounts({
          owner: wallet.publicKey, collateralVault: collateralVaultPda,
          lpPositionMint: lpMint, ownerLpAta, custodyAta: custodyAtaPda,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      await cmProgram.methods.updateValuation(new anchor.BN(200_000_000)) // $200
        .accounts({ authority: wallet.publicKey, collateralVault: collateralVaultPda })
        .rpc();
    });

    it("deposits USDC into vault", async () => {
      await program.methods.depositVaultLiquidity(new anchor.BN(500_000_000))
        .accounts({
          depositor: wallet.publicKey, lendingVault: lendingVaultPda,
          vaultUsdcAta, depositorUsdcAta: depositorAta, tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      const v = await program.account.lendingVault.fetch(lendingVaultPda);
      expect(v.totalLiquidity.toNumber()).to.eq(500_000_000);
    });

    // Run rejection test BEFORE creating the loan so loanPda doesn't exist yet.
    // When init+handler fails, the account creation is rolled back atomically.
    it("rejects loan above 50% LTV", async () => {
      try {
        await program.methods.issueLoan(new anchor.BN(101_000_000))
          .accounts({
            borrower: wallet.publicKey, lendingVault: lendingVaultPda,
            collateralVault: collateralVaultPda, loanAccount: loanPda,
            vaultUsdcAta, borrowerUsdcAta: borrowerAta,
            tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        const code = e?.error?.errorCode?.code ?? e?.message;
        expect(code).to.include("ExceedsMaxBorrow");
      }
    });

    it("issues loan up to 50% LTV", async () => {
      const balBefore = await getTokenBalance(provider.connection, borrowerAta);
      await program.methods.issueLoan(new anchor.BN(100_000_000)) // $100 = 50% of $200
        .accounts({
          borrower: wallet.publicKey, lendingVault: lendingVaultPda,
          collateralVault: collateralVaultPda, loanAccount: loanPda,
          vaultUsdcAta, borrowerUsdcAta: borrowerAta,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        })
        .rpc();

      const loan = await program.account.loanAccount.fetch(loanPda);
      expect(loan.principalUsd.toNumber()).to.eq(100_000_000);
      expect(loan.feeUsd.toNumber()).to.eq(100_000); // 0.1%
      const balAfter = await getTokenBalance(provider.connection, borrowerAta);
      expect(balAfter - balBefore).to.eq(100_000_000n);
    });
  });

  describe("repay_loan", () => {
    it("repays loan from borrower ATA and marks Repaid", async () => {
      await program.methods.repayLoan(new anchor.BN(100_100_000)) // principal + fee
        .accounts({
          caller: wallet.publicKey,
          lendingVault: lendingVaultPda,
          loanAccount: loanPda,
          vaultUsdcAta,
          repaymentSource: borrowerAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      const loan = await program.account.loanAccount.fetch(loanPda);
      expect(loan.status).to.deep.eq({ repaid: {} });

      const v = await program.account.lendingVault.fetch(lendingVaultPda);
      expect(v.totalLoansOutstanding.toNumber()).to.eq(0);
    });
  });
});
