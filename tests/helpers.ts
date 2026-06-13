import * as anchor from "@coral-xyz/anchor";
import {
  createMint, getAccount,
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress, createAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { Keypair, PublicKey, Connection } from "@solana/web3.js";

export async function createTestMint(
  provider: anchor.AnchorProvider,
  decimals = 6,
  authority?: Keypair,
): Promise<PublicKey> {
  const auth = authority ?? (provider.wallet as anchor.Wallet).payer;
  return createMint(
    provider.connection,
    auth,
    auth.publicKey,
    null,
    decimals,
  );
}

export async function createFundedAta(
  provider: anchor.AnchorProvider,
  mint: PublicKey,
  owner: PublicKey,
  amount: bigint,
  mintAuthority?: Keypair,
): Promise<PublicKey> {
  const auth = mintAuthority ?? (provider.wallet as anchor.Wallet).payer;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const ata = await createAssociatedTokenAccount(
    provider.connection, payer, mint, owner,
  );
  if (amount > 0n) {
    await mintTo(provider.connection, payer, mint, ata, auth, amount);
  }
  return ata;
}

export async function getTokenBalance(
  connection: Connection,
  ata: PublicKey,
): Promise<bigint> {
  const info = await getAccount(connection, ata);
  return info.amount;
}
