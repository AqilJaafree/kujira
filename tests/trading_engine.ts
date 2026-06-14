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
  const program   = anchor.workspace.TradingEngine    as Program<TradingEngine>;
  const lvProgram = anchor.workspace.LendingVault     as Program<LendingVault>;
  const cmProgram = anchor.workspace.CollateralManager as Program<CollateralManager>;
  const wallet    = provider.wallet as anchor.Wallet;

  const BN = anchor.BN;
  const market = anchor.web3.Keypair.generate().publicKey;

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

  // ER delegation PDAs (needed for open_session account validation)
  let bufferTradeSession:           PublicKey;
  let delegationRecordTradeSession: PublicKey;
  let delegationMetaTradeSession:   PublicKey;

  before(async () => {
    [lendingVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lending_vault")], lvProgram.programId,
    );
    [vaultUsdcAta] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_usdc"), lendingVaultPda.toBuffer()], lvProgram.programId,
    );

    // Reuse the vault's USDC mint from the lending_vault test suite
    const vaultState = await lvProgram.account.lendingVault.fetch(lendingVaultPda);
    usdcMint     = vaultState.usdcMint;
    depositorAta = getAssociatedTokenAddressSync(usdcMint, wallet.publicKey);

    if (vaultState.availableLiquidity.toNumber() < 200_000_000) {
      await lvProgram.methods.depositVaultLiquidity(new BN(500_000_000))
        .accounts({
          depositor: wallet.publicKey, lendingVault: lendingVaultPda,
          vaultUsdcAta, depositorUsdcAta: depositorAta, tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    }

    // Fresh LP mint + collateral vault + loan for this test suite
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

    await cmProgram.methods.updateValuation(new BN(200_000_000)) // $200 collateral
      .accounts({ authority: wallet.publicKey, collateralVault: collateralVaultPda })
      .rpc();

    await lvProgram.methods.issueLoan(new BN(100_000_000)) // $100 = 50% LTV
      .accounts({
        borrower: wallet.publicKey, lendingVault: lendingVaultPda,
        collateralVault: collateralVaultPda, loanAccount: loanPda,
        vaultUsdcAta, borrowerUsdcAta: depositorAta,
        tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
      })
      .rpc();

    // PDA for trade session
    [tradeSessionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("trade_session"), loanPda.toBuffer()],
      program.programId,
    );

    // ER delegation helper PDAs (for open_session validation tests)
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

  describe("open_session validation (pre-delegation checks)", () => {
    it("rejects leverage=0 (below minimum)", async () => {
      try {
        await program.methods.openSession(
          market, { long: {} }, new BN(50_000_000),
          0, Array(32).fill(0), new BN(100_000),
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
          market, { long: {} }, new BN(200_000_000), // $200 > $100 loan
          2, Array(32).fill(0), new BN(100_000),
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

  describe("stub_open_session → update_price → force_close_session", () => {
    it("creates TradeSession via stub (no ER delegation)", async () => {
      await program.methods
        .stubOpenSession(
          market, { long: {} }, new BN(50_000_000),
          2, Array(32).fill(0), new BN(100_000),
          new BN(0), // 0 = use normal 24h expiry
        )
        .accounts({
          payer: wallet.publicKey,
          tradeSession: tradeSessionPda,
          loanAccount: loanPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const ts = await program.account.tradeSession.fetch(tradeSessionPda);
      expect(ts.status).to.deep.eq({ open: {} });
      expect(ts.leverage).to.eq(2);
      expect(ts.sizeUsd.toNumber()).to.eq(50_000_000);
    });

    it("keeper updates current price on ER shard", async () => {
      const newPrice = new BN(105_000); // price moved +5%
      await program.methods.updatePrice(newPrice)
        .accounts({ keeper: wallet.publicKey, tradeSession: tradeSessionPda })
        .rpc();

      const ts = await program.account.tradeSession.fetch(tradeSessionPda);
      expect(ts.currentPrice.toNumber()).to.eq(105_000);
    });

    it("force_close_session rejects when not yet expired", async () => {
      try {
        await program.methods.forceCloseSession(new BN(-3_000_000))
          .accounts({ keeper: wallet.publicKey, tradeSession: tradeSessionPda })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        const code = e?.error?.errorCode?.code ?? e?.message;
        expect(code).to.include("SessionNotExpired");
      }
    });

    it("force_close_session succeeds on expired session (stub with past expiry)", async () => {
      // Need a second loan + fresh session with override_expires_at = 1 (far in the past)
      const lpMint2     = await createTestMint(provider);
      const ownerLpAta2 = await createFundedAta(provider, lpMint2, wallet.publicKey, 1n);
      const [cv2] = PublicKey.findProgramAddressSync(
        [Buffer.from("collateral_vault"), wallet.publicKey.toBuffer(), lpMint2.toBuffer()],
        cmProgram.programId,
      );
      const [custody2] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_custody"), cv2.toBuffer()], cmProgram.programId,
      );
      const [loan2] = PublicKey.findProgramAddressSync(
        [Buffer.from("loan"), cv2.toBuffer()], lvProgram.programId,
      );
      const [ts2Pda] = PublicKey.findProgramAddressSync(
        [Buffer.from("trade_session"), loan2.toBuffer()],
        program.programId,
      );

      await cmProgram.methods.depositCollateral({ meteora: {} })
        .accounts({
          owner: wallet.publicKey, collateralVault: cv2,
          lpPositionMint: lpMint2, ownerLpAta: ownerLpAta2, custodyAta: custody2,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .rpc();

      await cmProgram.methods.updateValuation(new BN(200_000_000))
        .accounts({ authority: wallet.publicKey, collateralVault: cv2 })
        .rpc();

      await lvProgram.methods.issueLoan(new BN(100_000_000))
        .accounts({
          borrower: wallet.publicKey, lendingVault: lendingVaultPda,
          collateralVault: cv2, loanAccount: loan2,
          vaultUsdcAta, borrowerUsdcAta: depositorAta,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Stub-open with expires_at = 1 (Unix epoch, deep in the past)
      await program.methods
        .stubOpenSession(
          market, { short: {} }, new BN(50_000_000),
          2, Array(32).fill(0), new BN(100_000),
          new BN(1), // already expired
        )
        .accounts({
          payer: wallet.publicKey,
          tradeSession: ts2Pda,
          loanAccount: loan2,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await program.methods.forceCloseSession(new BN(0))
        .accounts({ keeper: wallet.publicKey, tradeSession: ts2Pda })
        .rpc();

      const ts = await program.account.tradeSession.fetch(ts2Pda);
      expect(ts.status).to.deep.eq({ settling: {} });
    });
  });
});
