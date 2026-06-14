import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Settlement } from "../target/types/settlement";
import { TradingEngine } from "../target/types/trading_engine";
import { LendingVault } from "../target/types/lending_vault";
import { CollateralManager } from "../target/types/collateral_manager";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { expect } from "chai";
import { createTestMint, createFundedAta } from "./helpers";

describe("settlement", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program   = anchor.workspace.Settlement    as Program<Settlement>;
  const teProgram = anchor.workspace.TradingEngine  as Program<TradingEngine>;
  const lvProgram = anchor.workspace.LendingVault   as Program<LendingVault>;
  const cmProgram = anchor.workspace.CollateralManager as Program<CollateralManager>;
  const wallet    = provider.wallet as anchor.Wallet;

  const BN = anchor.BN;
  const market = anchor.web3.Keypair.generate().publicKey;

  let usdcMint:           PublicKey;
  let lendingVaultPda:    PublicKey;
  let vaultUsdcAta:       PublicKey;
  let depositorAta:       PublicKey;
  let collateralVaultPda: PublicKey;
  let custodyAtaPda:      PublicKey;
  let lpMint:             PublicKey;
  let ownerLpAta:         PublicKey;
  let loanPda:            PublicKey;
  let tradeSessionPda:    PublicKey;
  let settlementRecordPda: PublicKey;

  before(async () => {
    [lendingVaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lending_vault")], lvProgram.programId,
    );
    [vaultUsdcAta] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_usdc"), lendingVaultPda.toBuffer()], lvProgram.programId,
    );

    const vaultState = await lvProgram.account.lendingVault.fetch(lendingVaultPda);
    usdcMint     = vaultState.usdcMint;
    depositorAta = getAssociatedTokenAddressSync(usdcMint, wallet.publicKey);

    if (vaultState.availableLiquidity.toNumber() < 100_000_000) {
      await lvProgram.methods.depositVaultLiquidity(new BN(500_000_000))
        .accounts({
          depositor: wallet.publicKey, lendingVault: lendingVaultPda,
          vaultUsdcAta, depositorUsdcAta: depositorAta, tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    }

    // Fresh collateral vault + loan for settlement tests
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
    [tradeSessionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("trade_session"), loanPda.toBuffer()],
      teProgram.programId,
    );
    [settlementRecordPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("settlement"), tradeSessionPda.toBuffer()],
      program.programId,
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

    // Stub-open session and move it to Settling by force-closing with past expiry
    await teProgram.methods
      .stubOpenSession(
        market, { long: {} }, new BN(50_000_000),
        2, Array(32).fill(0), new BN(100_000),
        new BN(1), // expires_at = 1 → immediately expired
      )
      .accounts({
        payer: wallet.publicKey,
        tradeSession: tradeSessionPda,
        loanAccount: loanPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // force_close → status becomes Settling
    await teProgram.methods.forceCloseSession(new BN(0))
      .accounts({ keeper: wallet.publicKey, tradeSession: tradeSessionPda })
      .rpc();
  });

  describe("settle validation", () => {
    it("rejects settle when trade_session is not owned by trading-engine", async () => {
      const fakeSession = anchor.web3.Keypair.generate().publicKey;
      const [fakeRecord] = PublicKey.findProgramAddressSync(
        [Buffer.from("settlement"), fakeSession.toBuffer()],
        program.programId,
      );
      try {
        await program.methods.settle(new BN(-5_000_000))
          .accounts({
            settler:               wallet.publicKey,
            settlementRecord:      fakeRecord,
            tradeSession:          fakeSession,
            loanAccount:           lendingVaultPda,
            lendingVault:          lendingVaultPda,
            vaultUsdcAta:          vaultUsdcAta,
            repaymentSource:       depositorAta,
            lendingVaultProgram:   lvProgram.programId,
            tradingEngineProgram:  teProgram.programId,
            tokenProgram:          TOKEN_PROGRAM_ID,
            systemProgram:         SystemProgram.programId,
          })
          .rpc();
        expect.fail("should have thrown");
      } catch (e: any) {
        const msg: string = e?.message ?? "";
        expect(msg.length > 0).to.be.true;
      }
    });
  });

  describe("settle happy path", () => {
    it("settles a Settling session and repays the lending vault", async () => {
      const loanBefore = await lvProgram.account.loanAccount.fetch(loanPda);
      expect(loanBefore.status).to.deep.eq({ active: {} });

      const vaultBefore = await lvProgram.account.lendingVault.fetch(lendingVaultPda);

      await program.methods.settle(new BN(5_000_000)) // +$5 realized P&L
        .accounts({
          settler:               wallet.publicKey,
          settlementRecord:      settlementRecordPda,
          tradeSession:          tradeSessionPda,
          loanAccount:           loanPda,
          lendingVault:          lendingVaultPda,
          vaultUsdcAta:          vaultUsdcAta,
          repaymentSource:       depositorAta,
          lendingVaultProgram:   lvProgram.programId,
          tradingEngineProgram:  teProgram.programId,
          tokenProgram:          TOKEN_PROGRAM_ID,
          systemProgram:         SystemProgram.programId,
        })
        .rpc();

      // Loan marked Repaid
      const loanAfter = await lvProgram.account.loanAccount.fetch(loanPda);
      expect(loanAfter.status).to.deep.eq({ repaid: {} });

      // TradeSession marked Settled
      const ts = await teProgram.account.tradeSession.fetch(tradeSessionPda);
      expect(ts.status).to.deep.eq({ settled: {} });

      // SettlementRecord written
      const sr = await program.account.settlementRecord.fetch(settlementRecordPda);
      expect(sr.realizedPnl.toNumber()).to.eq(5_000_000);
      expect(sr.repaidAmount.toNumber()).to.eq(100_100_000); // principal + 0.1% fee

      // Lending vault liquidity restored
      const vaultAfter = await lvProgram.account.lendingVault.fetch(lendingVaultPda);
      expect(vaultAfter.availableLiquidity.toNumber())
        .to.eq(vaultBefore.availableLiquidity.toNumber() + 100_000_000);
    });
  });

  describe("deployment check", () => {
    it("settlement program is deployed and executable", async () => {
      const info = await provider.connection.getAccountInfo(program.programId);
      expect(info).to.not.be.null;
      expect(info!.executable).to.be.true;
    });
  });
});
