use ed25519_dalek::{Keypair, Signer};
use crate::types::{SettlementError, SignedTradeSettlement, TradeSettlementMessage};

pub struct EngineSigner {
    keypair: Keypair,
}

impl EngineSigner {
    pub fn from_env() -> Result<Self, SettlementError> {
        let hex_key = std::env::var("ENGINE_SIGNING_KEY")
            .map_err(|_| SettlementError::MissingKeypair)?;
        let bytes = hex::decode(&hex_key)
            .map_err(|_| SettlementError::InvalidKeypair)?;
        let keypair = Keypair::from_bytes(&bytes)
            .map_err(|_| SettlementError::InvalidKeypair)?;
        Ok(Self { keypair })
    }

    pub fn solana_pubkey(&self) -> solana_sdk::pubkey::Pubkey {
        solana_sdk::pubkey::Pubkey::new_from_array(self.keypair.public.to_bytes())
    }

    pub fn sign_trade_raw(
        &self,
        msg: &[u8],
        buyer_nonce: u64,
        seller_nonce: u64,
    ) -> Result<SignedTradeSettlement, SettlementError> {
        let signature = self.keypair.sign(msg);
        Ok(SignedTradeSettlement {
            signature: signature.to_bytes().to_vec(),
            buyer_nonce,
            seller_nonce,
        })
    }
}