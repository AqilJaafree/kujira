# Kujira — Product Requirements Document

**Version:** 1.1  
**Date:** 2026-06-14  
**Status:** Draft  
**Updated:** Corrected flash.trade integration (perpetuals, not spot); added Magicblock ER SDK patterns; added flash-trade MCP tooling

---

## 1. Overview

Kujira is a DeFi lending and trading platform on Solana that unlocks the idle capital locked inside LP positions. Users deposit their Meteora DLMM, Raydium, or Orca LP positions as collateral, borrow up to 50% of the position's value, and open leveraged perpetual positions on flash.trade — executed at sub-second speed via Magicblock Ephemeral Rollups (ER). When the position closes, repayment is triggered automatically and atomically as the ER session undelegates back to Solana base layer. The LP position earns fees throughout the entire lifecycle.

**Tagline:** *Your liquidity, twice as productive.*

---

## 2. Problem Statement

LP providers on Solana lock significant capital into AMM pools to earn trading fees. This capital is productive but illiquid — it cannot simultaneously be used for directional trading without withdrawing from the pool, forfeiting fees, and incurring impermanent loss at the worst moment.

Existing solutions are incomplete:
- **Kamino / MarginFi**: Allow borrowing against LP positions but have no integrated trading flow — users must manually route borrowed funds elsewhere.
- **Jupiter**: Aggregates swaps but offers no LP-backed credit facility.
- **Leverage protocols**: Require separate collateral deposits, forcing users to choose between earning LP yield and trading.

**The gap**: No protocol lets you borrow against your live LP position and trade, with automatic settlement, without ever touching the LP itself.

---

## 3. Solution

Kujira introduces a three-layer flow:

```
[LP Position] → [Collateral Manager] → [Lending Vault]
                                              ↓
                              delegate TradeSession PDA to ER
                                              ↓
                        [Magicblock Ephemeral Rollup Session]
                          (zero-fee, sub-second txs in ER)
                                              ↓
                          flash.trade open_position (perps)
                       [monitor P&L / trigger orders via MCP]
                                              ↓
                          flash.trade close_position call
                                              ↓
                     commit_and_undelegate → base layer settles
                                              ↓
                          [Settlement: repay loan + net P&L]
```

**Key properties:**
- LP never exits the pool — fees keep accruing during the trade lifecycle
- 50% LTV cap provides a 2x safety buffer against LP value fluctuation
- Magicblock ER handles high-frequency P&L monitoring and trigger order checks at zero fee
- Undelegation is atomic: ER state commits to base layer before settlement executes
- flash.trade perpetuals (not spot) enable long/short with leverage and native TP/SL via trigger orders
- flash-trade MCP server (`npx flash-trade-mcp`) exposes all required operations as structured tools

---

## 4. Core User Flow

### Happy Path

1. **Connect wallet** — User connects Solana wallet (Phantom, Backpack, Solflare)
2. **Select LP position** — Dashboard shows all detected LP positions (DLMM, Raydium LP, Orca Whirlpool) with current USD value
3. **Deposit as collateral** — User deposits LP NFT/position into Kujira's Collateral Manager program
4. **View borrow limit** — Platform displays: collateral value, max borrow (50% LTV), available liquidity in vault
5. **Open trade** — User selects perp market (from `get_markets`), direction (long/short), size (up to borrow limit), optional TP/SL, and submits
6. **ER delegation** — `TradeSession` PDA is delegated to Magicblock ER; `open_position` is called on flash.trade through the ER session
7. **Live monitoring** — ER runs zero-fee crank loop: `get_price` + `get_position` each slot; `place_trigger_order` fires TP/SL when hit
8. **Close position** — User-initiated `close_position` call or TP/SL trigger fires; ER executes `commit_and_undelegate`
9. **Settlement on base layer** — After undelegation finalizes, `settlement` program reads closed P&L, repays loan, sends net to wallet
10. **Withdraw collateral** — User can withdraw LP position once no active loans exist

### Edge Cases

| Scenario | Behavior |
|---|---|
| Trade profit < loan amount | Net loss deducted from user wallet; user must top up to repay |
| LP value drops below LTV threshold during trade | Liquidation warning sent; position closed automatically if LTV hits 75% |
| Magicblock session timeout | Trade settlement reverts to base layer; loan status preserved, user prompted to close manually |
| flash.trade liquidity insufficient | Trade rejected at submission, no loan issued |

---

## 5. Product Requirements

### 5.1 Collateral Management

| ID | Requirement | Priority |
|---|---|---|
| CM-01 | Support Meteora DLMM positions as collateral | P0 |
| CM-02 | Support Raydium CLMM/LP positions as collateral | P0 |
| CM-03 | Support Orca Whirlpool positions as collateral | P1 |
| CM-04 | Real-time LP position valuation via Pyth + Switchboard oracles | P0 |
| CM-05 | Max LTV: 50% of collateral USD value | P0 |
| CM-06 | Liquidation trigger at 75% LTV (collateral value drops) | P0 |
| CM-07 | LP position must remain in protocol's custody during active loan | P0 |
| CM-08 | LP fees continue accruing to original owner during custody | P0 |
| CM-09 | Liquidation bots receive 5% of recovered collateral value as incentive | P0 |

### 5.2 Lending Vault

| ID | Requirement | Priority |
|---|---|---|
| LV-01 | Issue loans in USDC | P0 |
| LV-02 | Issue loans in SOL | P1 |
| LV-03 | Fixed borrow fee: 0.1% flat per trade session | P0 |
| LV-04 | Vault liquidity sourced from Kujira liquidity providers (separate LP role) | P0 |
| LV-05 | Vault LP earns borrow fees proportional to share | P0 |
| LV-06 | Minimum loan: $10 USDC equivalent | P0 |
| LV-07 | Maximum loan per position: 50% of collateral value at time of borrow | P0 |

### 5.3 Trade Execution (Magicblock ER + flash.trade Perpetuals)

| ID | Requirement | Priority |
|---|---|---|
| TE-01 | Delegate `TradeSession` PDA to Magicblock ER using `ephemeral-rollups-sdk` `delegate_pda()` at trade start | P0 |
| TE-02 | Call flash.trade `open_position` via MCP tool inside ER session to open leveraged perp | P0 |
| TE-03 | Support long and short directions on all flash.trade markets (`get_markets` at boot) | P0 |
| TE-04 | ER crank loop runs `get_price` + `get_position` each slot to update live P&L on `TradeSession` account | P0 |
| TE-05 | Place native TP/SL via flash.trade `place_trigger_order` MCP tool at position open | P1 |
| TE-06 | On close: call `close_position` → then `MagicIntentBundleBuilder::commit_and_undelegate()` atomically; if ER finalization fails, loan remains Active and user prompted to settle manually | P0 |
| TE-07 | Maximum trade duration: 24 hours; keeper calls `force_close_session` which calls `close_position` then undelegates | P0 |
| TE-08 | Real-time P&L streamed from ER session via WebSocket subscription to `TradeSession` account | P0 |
| TE-09 | Use Magic Router (`https://devnet-router.magicblock.app`) for ER RPC; base layer uses standard Solana RPC | P0 |

### 5.4 Settlement

| ID | Requirement | Priority |
|---|---|---|
| SE-01 | Repay principal + 0.1% fee from trade proceeds automatically | P0 |
| SE-02 | If proceeds insufficient, deduct shortfall from connected wallet | P0 |
| SE-03 | If wallet insufficient, trigger partial collateral liquidation | P0 |
| SE-04 | Net P&L credited to user wallet within 1 Solana block of trade close | P0 |
| SE-05 | Emit settlement event log for user history | P0 |
| SE-06 | If flash.trade execution requires off-chain relayer (non-CPI), settlement signed by relayer must be verified on-chain via Ed25519 signature check | P0 |

---

## 6. Smart Contract Architecture

### 6.1 Programs Overview

```
kujira/
├── programs/
│   ├── collateral-manager/     # LP deposit, valuation, custody
│   ├── lending-vault/          # Loan issuance, repayment, vault LP
│   ├── trading-engine/         # Magicblock session mgmt, flash.trade routing
│   └── settlement/             # Atomic repayment, net P&L distribution
└── sdk/
    └── kujira-sdk/             # TypeScript client SDK
```

### 6.2 Program: `collateral-manager`

**Accounts:**
```
CollateralVault {
    owner: Pubkey,
    lp_position_mint: Pubkey,       // LP NFT or position account
    lp_protocol: Enum(Meteora, Raydium, Orca),
    collateral_usd_value: u64,      // lamport-precision, updated by oracle crank
    max_borrow_usd: u64,            // 50% of collateral_usd_value
    active_loan: Option<Pubkey>,    // ref to LoanAccount
    deposited_at: i64,
    bump: u8,
}
```

**Instructions:**
- `deposit_collateral(lp_position)` — Transfer LP position into PDA custody
- `withdraw_collateral()` — Return LP to owner if no active loan
- `update_valuation(oracle_price)` — Crank-callable, updates USD value + max_borrow
- `trigger_liquidation()` — Callable by liquidator bots when LTV ≥ 75%

**Oracle integration:**
- Meteora DLMM: bin price × bin liquidity per tick, aggregated via Pyth SOL/USDC feed
- Raydium CLMM: sqrt price from pool state × Pyth price
- Orca Whirlpool: tick array → Pyth price
- Valuation cranked every 10 seconds by keeper bot

### 6.3 Program: `lending-vault`

**Accounts:**
```
LendingVault {
    total_liquidity: u64,
    available_liquidity: u64,
    total_loans_outstanding: u64,
    borrow_fee_bps: u16,            // 10 = 0.1%
    vault_lp_mint: Pubkey,
    bump: u8,
}

LoanAccount {
    borrower: Pubkey,
    collateral_vault: Pubkey,
    principal_usd: u64,
    fee_usd: u64,
    opened_at: i64,
    expires_at: i64,                // opened_at + 24h
    status: Enum(Active, Repaid, Liquidated),
    bump: u8,
}
```

**Instructions:**
- `issue_loan(collateral_vault, amount)` — Validate LTV, transfer USDC to trading-engine PDA
- `repay_loan(loan_account, proceeds)` — Called by settlement program after trade close
- `deposit_vault_liquidity(amount)` — Vault LPs deposit USDC, receive vault LP tokens
- `withdraw_vault_liquidity(lp_tokens)` — Vault LPs redeem tokens for USDC + accrued fees
- `force_expire_loan()` — Crank-callable after 24h, triggers settlement

### 6.4 Program: `trading-engine`

**Responsibility:** Delegates `TradeSession` PDA to Magicblock ER, routes `open_position`/`close_position` calls to flash.trade via MCP, and triggers settlement CPI on undelegate.

**Cargo dependency:**
```toml
ephemeral-rollups-sdk = { version = "0.x", features = ["anchor"] }
```

**Accounts:**
```
// #[ephemeral] macro injects undelegation callback discriminator
#[ephemeral]
pub mod trading_engine { ... }

TradeSession {
    loan_account: Pubkey,
    flash_trade_position_id: [u8; 32],  // ID returned by open_position MCP call
    market: Pubkey,                      // flash.trade market pubkey
    direction: Enum(Long, Short),
    size_usd: u64,
    leverage: u8,                        // 1-10x
    entry_price: u64,
    current_price: u64,                  // updated by ER crank each slot
    take_profit: Option<u64>,
    stop_loss: Option<u64>,
    trigger_order_id: Option<[u8; 32]>, // flash.trade trigger order ID
    opened_at: i64,
    expires_at: i64,                     // opened_at + 24h
    status: Enum(Open, Closing, Settled),
    bump: u8,
}
```

**Instructions:**
```rust
// Base layer: delegate TradeSession to ER, issue open_position via MCP
#[delegate]
#[derive(Accounts)]
pub struct OpenSession<'info> { payer, trade_session (mut, del), loan_account }

pub fn open_session(ctx, market, direction, size_usd, leverage, tp, sl) -> Result<()> {
    // 1. Validate loan_account is Active, size_usd ≤ loan principal
    // 2. Call flash-trade MCP open_position (off-chain relayer signs, verified on-chain)
    // 3. Store returned position_id + trigger_order_id in TradeSession
    // 4. delegate_pda() transfers TradeSession ownership to ER
}

// ER only: crank updates price + P&L
pub fn update_price(ctx, price: u64) -> Result<()> { ... }

// ER only: user-initiated or TP/SL keeper; undelegates back to base layer
pub fn close_session(ctx) -> Result<()> {
    // 1. Call flash-trade MCP close_position
    // 2. MagicIntentBundleBuilder::commit_and_undelegate(&[trade_session])
    // → triggers settlement CPI after finalization on base layer
}

// Base layer: crank-callable after 24h expiry
pub fn force_close_session(ctx) -> Result<()> { /* same as close_session */ }
```

**Dual-connection pattern (client SDK):**
```typescript
const baseConn = new Connection("https://api.devnet.solana.com");
const erConn   = new Connection("https://devnet-router.magicblock.app");

// Base layer: delegate
await program.methods.openSession(...).accounts({...})
  .remainingAccounts([{ pubkey: ER_VALIDATOR, isSigner: false, isWritable: false }])
  .sendAndConfirm(baseConn);                // sends to base layer

// ER: price updates (zero-fee, sub-second)
const erTx = await program.methods.updatePrice(price).transaction();
erTx.recentBlockhash = (await erConn.getLatestBlockhash()).blockhash;
await erConn.sendAndConfirm(erTx);          // sends to ER

// ER: close (commit_and_undelegate fires here)
await program.methods.closeSession().sendAndConfirm(erConn);
```

**ER Validators (devnet):**
- Asia: `MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57`
- EU: `MEUGGrYPxKk17hCr7wpT6s8dtNokZj5U2L57vjYMS8e`
- US: `MUS3hc9TCw4cGC12vHNoYcCGzJG1txjgQLZWVoeNHNd`

### 6.5 Flash-Trade MCP Integration

The `flash-trade-mcp` server (`npx flash-trade-mcp`, env `FLASH_API_URL=https://flashapi.trade`) provides the following tools used by Kujira's off-chain keeper and frontend:

| MCP Tool | Used by | Purpose |
|---|---|---|
| `get_markets` | Boot / frontend | List all perp markets; populate market selector |
| `get_market` | Frontend | Market details (min size, leverage caps) |
| `get_prices` | ER crank | Real-time price feed for P&L + LTV monitoring |
| `get_price` | ER crank | Single-market price |
| `get_pools` / `get_pool_data` | Frontend | Pool liquidity depth check before issuing loan |
| `open_position` | Keeper on `open_session` | Open leveraged perp on flash.trade |
| `close_position` | Keeper on `close_session` | Close perp position, returns realized P&L |
| `place_trigger_order` | Keeper on open | Set TP/SL trigger orders |
| `cancel_trigger_order` | Keeper on close | Cancel outstanding TP/SL before manual close |
| `get_position` | ER crank | Fetch current position size + unrealized P&L |
| `get_positions` | Frontend | User's open positions dashboard |
| `get_account_summary` | Frontend | Account equity, margin, available balance |
| `reverse_position` | Frontend (P1) | Flip long↔short without closing |
| `preview_limit_order_fees` | Frontend (P1) | Fee estimation before trade open |

**Keeper architecture:** An off-chain keeper process (Node.js) holds the flash-trade MCP client, signs MCP calls with a protocol-owned keypair, and submits the resulting transactions to the ER. The on-chain `trading-engine` program verifies the keeper's Ed25519 signature on any flash.trade instruction before accepting state updates (SE-06).

### 6.6 Program: `settlement`

**Responsibility:** Single-purpose atomic settlement. Called via CPI from trading-engine on trade close.

**Instructions:**
- `settle(trade_session, loan_account, collateral_vault)`:
  1. Calculate: `net = trade_proceeds - (principal + fee)`
  2. If `net >= 0`: repay loan from proceeds, send net to borrower wallet
  3. If `net < 0`: repay loan from proceeds + borrower wallet; if still insufficient → liquidate partial collateral
  4. Mark `LoanAccount.status = Repaid`
  5. Release `CollateralVault.active_loan = None`
  6. Emit `SettlementEvent`

---

## 7. Frontend Architecture

### 7.1 Tech Stack

| Layer | Choice | Reason |
|---|---|---|
| Framework | Next.js 14 (App Router) | SSR for fast initial load, API routes for keeper calls |
| Styling | Tailwind CSS + shadcn/ui | Rapid UI with accessible primitives |
| Wallet | `@solana/wallet-adapter-react` | Standard Solana wallet integration |
| On-chain reads | `@solana/web3.js` + Anchor client | Deserialize program accounts |
| Price feeds | Pyth Network SDK | Real-time oracle prices for LP valuation display |
| State | Zustand | Lightweight, no boilerplate |
| Charts | TradingView Lightweight Charts | P&L and price charts |

### 7.2 Page Structure

```
/                       → Landing page (hero, how it works, stats)
/app                    → Main dashboard (requires wallet)
/app/collateral         → Deposit / withdraw LP positions
/app/borrow             → View borrow limit, open loan
/app/trade              → Trade interface (open/close positions)
/app/history            → Settlement history, past trades
/app/vault              → Vault LP deposit/withdraw (liquidity providers)
```

### 7.3 Key Components

**`<CollateralCard />`**
- Detects user's LP positions across Meteora, Raydium, Orca via wallet scan
- Displays: protocol logo, token pair, current USD value, max borrow, status (idle / deposited / active loan)
- Actions: Deposit / Withdraw

**`<BorrowPanel />`**
- Slider: 0 → max borrow (50% LTV)
- Shows: borrow amount, flat fee (0.1%), net amount to trade
- Warning banner if LP value is within 30% of liquidation threshold
- CTA: "Open Trade"

**`<TradeInterface />`**
- Perp market selector (populated from `get_markets` MCP tool at page load)
- Long / Short toggle
- Size input in USDC (capped at borrow amount)
- Leverage slider (1–10x, capped by flash.trade market max from `get_market`)
- TP / SL price inputs (optional; sent to `place_trigger_order` on open)
- Real-time price from `get_price` MCP tool (polled every 2s)
- Fee preview from `preview_limit_order_fees` before submit
- "Open Position" → triggers `open_session` instruction + keeper calls `open_position` MCP

**`<ActiveTrade />`**
- Live P&L ticker: WebSocket subscription to `TradeSession` PDA (ER updates `current_price` each slot)
- Entry price, mark price (`get_price`), liquidation price, unrealized P&L (`get_position`)
- Active TP/SL display from `get_orders`
- "Close Trade" → `cancel_trigger_order` (clear TP/SL) → `close_session` → settlement
- Countdown timer to 24h expiry

**`<SettlementModal />`**
- Shown on trade close
- Breakdown: Trade P&L | Loan repaid | Fee | Net to wallet
- Confetti if profitable

### 7.4 Data Flow

```
Wallet connects
    ↓
Scan wallet for LP positions (Meteora API + Raydium SDK + Orca SDK)
    ↓
Fetch CollateralVault PDAs for wallet pubkey (base layer RPC)
    ↓
Fetch oracle prices (Pyth) → compute current LTV
    ↓
flash-trade MCP: get_markets → populate market selector
    ↓
Display dashboard state
    ↓
User submits trade →
  base layer: open_session (delegate TradeSession to ER)
  keeper: flash-trade MCP open_position + place_trigger_order
    ↓
ER crank: get_price each slot → update TradeSession.current_price (zero-fee)
    ↓
WebSocket sub to TradeSession PDA (via Magic Router) → live P&L in UI
    ↓
Trade close (user / TP-SL / 24h expiry) →
  keeper: cancel_trigger_order → close_position MCP
  ER: commit_and_undelegate → base layer finalizes
    ↓
SettlementEvent → settlement program repays loan → refresh dashboard
```

---

## 8. Risk Management

| Risk | Mitigation |
|---|---|
| LP value drops > 50% during active trade | Liquidation bot monitors LTV every block; triggers `force_close_session` + partial collateral sale at 75% LTV |
| Magicblock session fails to finalize | 24h expiry on all loans; keeper bot force-closes stale sessions; loan state preserved on base layer |
| flash.trade insufficient liquidity | Pre-check pool depth before issuing loan; reject trade if slippage would exceed 1% |
| Oracle manipulation | Use TWA (time-weighted average) price over 30-second window for LTV checks; single-block spikes ignored |
| Smart contract bugs | Audit before mainnet; launch with per-position borrow cap of $5,000 USDC |
| Vault liquidity crunch | Utilization rate cap at 80% of vault; no new loans above 80% until repayments reduce utilization |

---

## 9. MVP Scope (Hackathon)

**In scope:**
- Meteora DLMM collateral only (Raydium/Orca in v2)
- USDC loans only
- `open_position` + `close_position` via flash-trade MCP (market orders, manual close)
- Magicblock ER for price-update crank loop + `commit_and_undelegate` on close
- MCP config: `.mcp.json` at project root (`npx flash-trade-mcp`, `FLASH_API_URL=https://flashapi.trade`)
- Devnet deployment (Magic Router: `https://devnet-router.magicblock.app`)

**Out of scope for v1:**
- Raydium / Orca collateral
- SOL-denominated loans
- Limit orders / TP / SL
- Vault LP token (hardcode protocol-owned vault for hackathon)
- Mobile UI

---

## 10. Success Metrics

| Metric | Target (30 days post-launch) |
|---|---|
| Total Value Locked (collateral) | $500K |
| Total loan volume | $250K |
| Unique borrowers | 200 |
| Average trade duration | < 4 hours |
| Loan default rate | < 0.5% |
| Protocol fee revenue | $250 (0.1% × volume) |
| Vault LP yield (annualized) | > 15% APY |

---

## 11. Open Questions

1. **Magicblock ER commit latency**: `commit_and_undelegate` finalizes asynchronously. Need to measure worst-case slots between ER close and base layer finalization to bound the settlement delay window.
2. **flash.trade keeper keypair custody**: The keeper process signs `open_position`/`close_position` MCP calls with a protocol keypair. Need to decide: HSM, multi-sig, or threshold scheme. Insecure keeper = drained positions.
3. **DLMM valuation precision**: Meteora DLMM positions span multiple bins. Need to confirm whether Meteora's SDK exposes a single `getPositionValue(pubkey)` call or requires manual bin aggregation across active bins.
4. **flash.trade position size limits**: `get_market` returns per-market max position sizes. Keeper must enforce these against the borrow amount at `open_session` time, or `open_position` will reject.
5. **ER validator selection**: Devnet has 3 validators (Asia/EU/US). Production validator set TBD — need to confirm mainnet ER validator availability with Magicblock before launch.

---

*End of PRD v1.0*
