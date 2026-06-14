import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CollateralManager } from "../target/types/collateral_manager";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { expect } from "chai";
import { createTestMint, createFundedAta, getTokenBalance } from "./helpers";

describe("collateral-manager", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.CollateralManager as Program<CollateralManager>;
  const wallet = provider.wallet as anchor.Wallet;

  let lpMint: PublicKey;
  let ownerLpAta: PublicKey;
  let collateralVaultPda: PublicKey;
  let custodyAtaPda: PublicKey;

  before(async () => {
    lpMint = await createTestMint(provider);
    ownerLpAta = await createFundedAta(provider, lpMint, wallet.publicKey, 1n);

    [collateralVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("collateral_vault"), wallet.publicKey.toBuffer(), lpMint.toBuffer()],
      program.programId,
    );
    [custodyAtaPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_custody"), collateralVaultPda.toBuffer()],
      program.programId,
    );
  });

  describe("deposit_collateral", () => {
    it("deposits LP token into custody and initialises vault", async () => {
      await program.methods
        .depositCollateral({ meteora: {} })
        .accounts({
          owner: wallet.publicKey,
          collateralVault: collateralVaultPda,
          lpPositionMint: lpMint,
          ownerLpAta,
          custodyAta: custodyAtaPda,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      const vault = await program.account.collateralVault.fetch(collateralVaultPda);
      expect(vault.owner.toString()).to.eq(wallet.publicKey.toString());
      expect(vault.lpPositionMint.toString()).to.eq(lpMint.toString());
      expect(vault.activeLoan).to.be.null;
      expect(vault.collateralUsdValue.toNumber()).to.eq(0);

      const custodyBal = await getTokenBalance(provider.connection, custodyAtaPda);
      expect(custodyBal).to.eq(1n);

      const ownerBal = await getTokenBalance(provider.connection, ownerLpAta);
      expect(ownerBal).to.eq(0n);
    });
  });

  describe("withdraw_collateral", () => {
    it("returns LP token to owner and closes vault", async () => {
      await program.methods
        .withdrawCollateral()
        .accounts({
          owner: wallet.publicKey,
          collateralVault: collateralVaultPda,
          custodyAta: custodyAtaPda,
          ownerLpAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const ownerBal = await getTokenBalance(provider.connection, ownerLpAta);
      expect(ownerBal).to.eq(1n);

      const vaultInfo = await provider.connection.getAccountInfo(collateralVaultPda);
      expect(vaultInfo).to.be.null;
    });
  });

  describe("update_valuation + trigger_liquidation", () => {
    let vaultPda: PublicKey;
    let custodyPda: PublicKey;
    let mint3: PublicKey;
    let ata3: PublicKey;

    before(async () => {
      mint3 = await createTestMint(provider);
      ata3  = await createFundedAta(provider, mint3, wallet.publicKey, 1n);
      [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("collateral_vault"), wallet.publicKey.toBuffer(), mint3.toBuffer()],
        program.programId,
      );
      [custodyPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_custody"), vaultPda.toBuffer()], program.programId,
      );
      await program.methods.depositCollateral({ meteora: {} })
        .accounts({
          owner: wallet.publicKey, collateralVault: vaultPda, lpPositionMint: mint3,
          ownerLpAta: ata3, custodyAta: custodyPda, tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId, rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();
    });

    it("sets usd_value and max_borrow at 50% LTV", async () => {
      await program.methods.updateValuation(new anchor.BN(100_000_000))
        .accounts({ authority: wallet.publicKey, collateralVault: vaultPda })
        .rpc();

      const v = await program.account.collateralVault.fetch(vaultPda);
      expect(v.collateralUsdValue.toNumber()).to.eq(100_000_000);
      expect(v.maxBorrowUsd.toNumber()).to.eq(50_000_000);
    });

    it("trigger_liquidation fails when LTV is healthy", async () => {
      // Reuse ata3 — wallet.publicKey already has an ATA for mint3
      const liquidatorAta = ata3;
      try {
        await program.methods.triggerLiquidation(new anchor.BN(10_000_000))
          .accounts({
            liquidator: wallet.publicKey, collateralVault: vaultPda,
            custodyAta: custodyPda, liquidatorAta, tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.error.errorCode.code).to.eq("LtvHealthy");
      }
    });

    it("trigger_liquidation succeeds when LTV >= 75%", async () => {
      const liquidatorAta = ata3;
      await program.methods.triggerLiquidation(new anchor.BN(80_000_000))
        .accounts({
          liquidator: wallet.publicKey, collateralVault: vaultPda,
          custodyAta: custodyPda, liquidatorAta, tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const liquidatorBal = await getTokenBalance(provider.connection, liquidatorAta);
      expect(liquidatorBal).to.eq(1n);
    });
  });
});
