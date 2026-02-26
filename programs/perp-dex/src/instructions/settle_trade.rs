use anchor_lang::prelude::*;
use solana_program::sysvar::instructions::{load_instruction_at_checked, ID as IX_SYSVAR_ID};
use crate::state::*;
use crate::error::PerpError;

#[derive(Accounts)]
#[instruction(trade_id: u64)]
pub struct SettleTrade<'info> {
    #[account(seeds = [b"engine_config"], bump = config.bump)]
    pub config: Account<'info, EngineConfig>,

    #[account(mut, seeds = [b"margin_account", buyer_margin.owner.as_ref()], bump = buyer_margin.bump)]
    pub buyer_margin: Account<'info, MarginAccount>,

    #[account(mut, seeds = [b"margin_account", seller_margin.owner.as_ref()], bump = seller_margin.bump)]
    pub seller_margin: Account<'info, MarginAccount>,

    /// CHECK: Instructions Sysvar for Ed25519 introspection
    #[account(address = IX_SYSVAR_ID)]
    pub ix_sysvar: AccountInfo<'info>,
}

pub fn settle_trade_handler(
    ctx: Context<SettleTrade>, 
    _trade_id: u64, 
    price: u64, 
    qty: u64, 
    b_nonce: u64, 
    s_nonce: u64
) -> Result<()> {
    // 1. Signature Verification (Introspection)
    let ix_sysvar = &ctx.accounts.ix_sysvar;
    let current_ix = solana_program::sysvar::instructions::load_current_index_checked(ix_sysvar)?;
    require!(current_ix > 0, PerpError::MissingSignature);

    let signature_ix = load_instruction_at_checked((current_ix - 1) as usize, ix_sysvar)?;
    require!(signature_ix.program_id == solana_program::ed25519_program::ID, PerpError::InvalidSignatureProgram);
    
    // 2. Replay Protection (Nonces)
    let b_account = &mut ctx.accounts.buyer_margin;
    let s_account = &mut ctx.accounts.seller_margin;
    
    require!(b_nonce == b_account.nonce, PerpError::StaleNonce);
    require!(s_nonce == s_account.nonce, PerpError::StaleNonce);

    // 3. Execution: Apply fills to both accounts
    // For Perp DEX: Buyer gets +qty (Long), Seller gets -qty (Short)
    let market = [0u8; 16]; // In production, pass market name as bytes
    
    apply_fill_to_account(b_account, market, qty as i64, price)?;
    apply_fill_to_account(s_account, market, -(qty as i64), price)?;

    // 4. Advance Nonces
    b_account.nonce += 1;
    s_account.nonce += 1;

    msg!("Settled Trade {}: Price {} | Qty {}", _trade_id, price, qty);
    Ok(())
}

fn apply_fill_to_account(
    account: &mut MarginAccount,
    market: [u8; 16],
    qty_delta: i64, 
    price: u64,
) -> Result<()> {
    let position_idx = account.positions.iter().position(|p| p.market == market);

    match position_idx {
        Some(idx) => {
            let pos = &mut account.positions[idx];
            
            // --- PnL Calculation ---
            // If reducing or flipping a position, realize PnL
            if pos.size != 0 && (pos.size > 0) != (qty_delta > 0) {
                let closed_qty = pos.size.abs().min(qty_delta.abs());
                let pnl = if pos.size > 0 {
                    (price as i64 - pos.avg_entry_price as i64) * closed_qty
                } else {
                    (pos.avg_entry_price as i64 - price as i64) * closed_qty
                };
                
                // Adjust collateral (6 decimal precision)
                let collateral_change = pnl / 1_000_000;
                account.collateral = if collateral_change >= 0 {
                    account.collateral.checked_add(collateral_change as u64).unwrap()
                } else {
                    account.collateral.checked_sub(collateral_change.unsigned_abs()).unwrap()
                };
            }
            
            // --- Average Entry Price Calculation ---
            let new_size = pos.size + qty_delta;
            if new_size != 0 && (new_size > 0) == (pos.size > 0) {
                 // Weighted average for increasing position size
                 pos.avg_entry_price = (pos.avg_entry_price * pos.size.unsigned_abs() + price * qty_delta.unsigned_abs()) 
                                        / new_size.unsigned_abs() as u64;
            } else if new_size != 0 {
                // If flipping position (Short to Long or vice versa), new entry is the fill price
                pos.avg_entry_price = price;
            }
            
            pos.size = new_size;

            // If position is closed, clean up the slot
            if pos.size == 0 {
                *pos = Position::default();
                account.position_count -= 1;
            }
        }
        None => {
            // Open new position
            let count = account.position_count as usize;
            require!(count < MAX_POSITIONS, PerpError::MaxPositionsReached);
            account.positions[count] = Position { market, size: qty_delta, avg_entry_price: price };
            account.position_count += 1;
        }
    }
    
    // --- Final Safety Check: Leverage (The Oracle Check) ---
    // Here, we ensure the user's total position value does not exceed their collateral * max_leverage.
    
    let mut total_notional: u64 = 0;
    for i in 0..account.position_count as usize {
        let pos = &account.positions[i];
        // notional = size * price / decimals
        let pos_notional = (pos.size.unsigned_abs() * price) / 1_000_000;
        total_notional = total_notional.checked_add(pos_notional).ok_or(PerpError::MathOverflow)?;
    }

    // Require: Collateral * MaxLeverage >= TotalNotional
    let max_leverage = 10u64;
    require!(
        account.collateral.checked_mul(max_leverage).unwrap_or(0) >= total_notional,
        PerpError::InsufficientCollateral
    );
    
    Ok(())
}