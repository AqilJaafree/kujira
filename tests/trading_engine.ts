import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TradingEngine } from "../target/types/trading_engine";
import { LendingVault } from "../target/types/lending_vault";
import { CollateralManager } from "../target/types/collateral_manager";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { expect } from "chai";
import { createTestMint, createFundedAta } from "./helpers";

// Delegation program (Magicblock) — must be deployed for open_session to succeed
const DELEGATION_PROGRAM_ID = new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");

describe("trading-engine", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program   = anchor.workspace.TradingEngine  as Program<TradingEngine>;
  const lvProgram = anchor.workspace.LendingVault   as Program<LendingVault>;
  const cmProgram = anchor.workspace.CollateralManager as Program<CollateralManager>;
  const wallet    = provider.wallet as anchor.Wallet;

  let usdcMint:           PublicKey;
  let lpMint:             PublicKey;
  let ownerLpAta:         PublicKey;
  let collateralVaultPda: PublicKey;
  let custodyAtaPda:      PublicKey;
  let lendingVaultPda:    PublicKey;
  let vaultUsdcAta:       PublicKey;
  let depositorAta:       PublicKey;
  let loanPda:            PublicKey;
  let tradeSessionPda:    PublicKey;
  let bufferTradeSession:            PublicKey;
  let delegationRecordTradeSession:  PublicKey;
  let delegationMetaTradeSession:    PublicKey;

  const BN = anchor.BN;
  const market = anchor.web3.Keypair.generate().publicKey;

  before(async () => {
    [lendingVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lending_vault")], lvProgram.programId,
    );
    [vaultUsdcAta] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_usdc"), lendingVaultPda.toBuffer()], lvProgram.programId,
    );

    // Use the vault's mint so constraint borrower_usdc_ata.mint == lending_vault.usdc_mint passes
    const vaultState = await lvProgram.account.lendingVault.fetch(lendingVaultPda);
    usdcMint = vaultState.usdcMint;

    // ATA already funded from lending_vault test suite — just compute its address
    depositorAta = getAssociatedTokenAddressSync(usdcMint, wallet.publicKey);

    // Deposit more liquidity if vault doesn't have enough
    if (vaultState.availableLiquidity.toNumber() < 100_000_000) {
      await lvProgram.methods.depositVaultLiquidity(new BN(500_000_000))
        .accounts({
          depositor: wallet.publicKey, lendingVault: lendingVaultPda,
          vaultUsdcAta, depositorUsdcAta: depositorAta, tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    }

    // LP collateral
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
      [Buffer.from("loan"), collateralVaultPda.toBuffer()], lvProgram.programId,
    );

    await cmProgram.methods.depositCollateral({ meteora: {} })
      .accounts({
        owner: wallet.publicKey, collateralVault: collateralVaultPda,
        lpPositionMint: lpMint, ownerLpAta, custodyAta: custodyAtaPda,
        tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    await cmProgram.methods.updateValuation(new BN(200_000_000))
      .accounts({ authority: wallet.publicKey, collateralVault: collateralVaultPda })
      .rpc();

    await lvProgram.methods.issueLoan(new BN(100_000_000))
      .accounts({
        borrower: wallet.publicKey, lendingVault: lendingVaultPda,
        collateralVault: collateralVaultPda, loanAccount: loanPda,
        vaultUsdcAta, borrowerUsdcAta: depositorAta,
        tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Derive trade_session PDA and delegation helper PDAs
    [tradeSessionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("trade_session"), loanPda.toBuffer()],
      program.programId,
    );
    [bufferTradeSession] = PublicKey.findProgramAddressSync(
      [Buffer.from("buffer"), tradeSessionPda.toBuffer()],
      program.programId,
    );
    [delegationRecordTradeSession] = PublicKey.findProgramAddressSync(
      [Buffer.from("delegation"), tradeSessionPda.toBuffer()],
      DELEGATION_PROGRAM_ID,
    );
    [delegationMetaTradeSession] = PublicKey.findProgramAddressSync(
      [Buffer.from("delegation-metadata"), tradeSessionPda.toBuffer()],
      DELEGATION_PROGRAM_ID,
    );
  });

  // Helper to build the open_session account map
  function openSessionAccounts() {
    return {
      payer: wallet.publicKey,
      bufferTradeSession,
      delegationRecordTradeSession,
      delegationMetadataTradeSession: delegationMetaTradeSession,
      tradeSession: tradeSessionPda,
      loanAccount: loanPda,
      systemProgram: SystemProgram.programId,
      ownerProgram: program.programId,
      delegationProgram: DELEGATION_PROGRAM_ID,
    };
  }

  describe("open_session validation", () => {
    it("rejects leverage=0 (below minimum)", async () => {
      try {
        await program.methods.openSession(
          market, { long: {} }, new BN(50_000_000),
          0 /* leverage */, Array(32).fill(0), new BN(100_000),
        )
          .accounts(openSessionAccounts())
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        const code = e?.error?.errorCode?.code ?? e?.message;
        expect(code).to.include("InvalidLeverage");
      }
    });

    it("rejects size exceeding loan principal", async () => {
      try {
        await program.methods.openSession(
          market, { long: {} }, new BN(200_000_000) /* $200 > $100 loan */,
          2 /* leverage */, Array(32).fill(0), new BN(100_000),
        )
          .accounts(openSessionAccounts())
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        const code = e?.error?.errorCode?.code ?? e?.message;
        expect(code).to.include("SizeExceedsLoan");
      }
    });
  });
});
