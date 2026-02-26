use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    sysvar,
    transaction::Transaction,
};
use anchor_lang::InstructionData;
use common_utils::MatchResult;
use anyhow::Result;
use rust_decimal::prelude::ToPrimitive;

pub mod signer;
pub mod types;

use crate::signer::EngineSigner;
use crate::types::*;

pub struct SettlementClient {
    pub rpc: RpcClient,
    pub relayer_fee_payer: Keypair,
}

impl SettlementClient {
    pub async fn settle_trade(
        &self,
        match_res: &MatchResult,
        engine_signer: &EngineSigner,
        program_id: &Pubkey,
    ) -> Result<String> {
        // 1. Derive necessary PDAs
        let (config_pda, _) = Pubkey::find_program_address(&[b"engine_config"], program_id);
        
        let b_pubkey = Pubkey::try_from(match_res.buyer_id.as_str())?;
        let s_pubkey = Pubkey::try_from(match_res.seller_id.as_str())?;

        let (b_margin_pda, _) = Pubkey::find_program_address(&[b"margin_account", b_pubkey.as_ref()], program_id);
        let (s_margin_pda, _) = Pubkey::find_program_address(&[b"margin_account", s_pubkey.as_ref()], program_id);

        // 2. Prepare Message for Signing
        let mut msg = Vec::new();
        msg.extend_from_slice(&match_res.trade_id.to_le_bytes());
        msg.extend_from_slice(b_pubkey.as_ref());
        msg.extend_from_slice(s_pubkey.as_ref());
        msg.extend_from_slice(&[0u8; 16]); // Market placeholder
        
        let p_u64 = (match_res.price * rust_decimal::Decimal::from(1_000_000)).to_u64().unwrap();
        let q_u64 = (match_res.quantity * rust_decimal::Decimal::from(1_000_000)).to_u64().unwrap();
        
        msg.extend_from_slice(&p_u64.to_le_bytes());
        msg.extend_from_slice(&q_u64.to_le_bytes());
        msg.extend_from_slice(&chrono::Utc::now().timestamp().to_le_bytes());

        // 3. Generate Signatures
        let signed_trade = engine_signer.sign_trade_raw(&msg, 0, 0)
            .map_err(|e| anyhow::anyhow!("Signing failed: {:?}", e))?;

        // 4. Build Instructions
        let verify_ix = self.build_verify_ix(&engine_signer.solana_pubkey(), &signed_trade.signature, &msg);

        let settle_ix = Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(b_margin_pda, false),
                AccountMeta::new(s_margin_pda, false),
                AccountMeta::new_readonly(sysvar::instructions::ID, false),
            ],
            data: hybrid_perp_dex::instruction::SettleTrade {
                trade_id: match_res.trade_id,
                price: p_u64,
                quantity: q_u64,
                buyer_nonce: 0,
                seller_nonce: 0,
            }.data(),
        };

        // 5. Broadcast
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[verify_ix, settle_ix],
            Some(&self.relayer_fee_payer.pubkey()),
            &[&self.relayer_fee_payer],
            blockhash,
        );

        let sig = self.rpc.send_and_confirm_transaction(&tx).await?;
        Ok(sig.to_string())
    }

    fn build_verify_ix(&self, pubkey: &Pubkey, sig: &[u8], msg: &[u8]) -> Instruction {
        let mut instruction_data = Vec::with_capacity(176 + msg.len());
        instruction_data.extend_from_slice(&[1, 0]); 
        instruction_data.extend_from_slice(&[112u8, 0]); 
        instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
        instruction_data.extend_from_slice(&[48u8, 0]);  
        instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
        instruction_data.extend_from_slice(&[176u8, 0]); 
        instruction_data.extend_from_slice(&(msg.len() as u16).to_le_bytes());
        instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
        
        instruction_data.extend_from_slice(&pubkey.to_bytes()); 
        instruction_data.extend_from_slice(sig);    
        instruction_data.extend_from_slice(msg);          

        Instruction {
            program_id: solana_sdk::ed25519_program::ID,
            accounts: vec![],
            data: instruction_data,
        }
    }
}