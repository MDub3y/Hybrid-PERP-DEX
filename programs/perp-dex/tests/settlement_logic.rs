use litesvm::LiteSVM;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
    sysvar,
};
use anchor_lang::{InstructionData, AccountDeserialize};
use hybrid_perp_dex::instruction as perp_ix;

#[test]
fn test_signature_and_margin_health() {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    let engine_identity = Keypair::new();
    
    // 1. Setup Environment
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let program_id = hybrid_perp_dex::id();
    
    // Ensure this path matches your build output (usually hybrid_perp_dex.so)
    let program_bytes = std::fs::read("../../target/deploy/hybrid_perp_dex.so")
        .expect("Compiled .so not found. Run 'anchor build' first.");
    svm.add_program(program_id, &program_bytes);

    // 2. Derive PDAs
    let (config_pda, _config_bump) = Pubkey::find_program_address(&[b"engine_config"], &program_id);
    
    let buyer = Keypair::new();
    let seller = Keypair::new();
    let (buyer_margin_pda, _) = Pubkey::find_program_address(&[b"margin_account", buyer.pubkey().as_ref()], &program_id);
    let (seller_margin_pda, _) = Pubkey::find_program_address(&[b"margin_account", seller.pubkey().as_ref()], &program_id);

    // 3. Initialize Protocol and Create Accounts
    let init_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false), // usdc_mint mock
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: perp_ix::Initialize { engine_signer: engine_identity.pubkey() }.data(),
    };

    let create_buyer_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(buyer_margin_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: perp_ix::CreateMarginAccount {}.data(),
    };

    let create_seller_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(seller_margin_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: perp_ix::CreateMarginAccount {}.data(),
    };

    let setup_tx = Transaction::new_signed_with_payer(
        &[init_ix, create_buyer_ix, create_seller_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );
    svm.send_transaction(setup_tx).expect("Failed to setup test accounts");

    // 4. Mock the "Match" and Signature
    let trade_id = 42u64;
    let price = 150_000_000u64; 
    let qty = 1_000_000u64;      
    let timestamp = 1620000000i64;
    
    // RECONSTRUCT PAYLOAD (Must match program logic exactly)
    let mut msg = Vec::new();
    msg.extend_from_slice(&trade_id.to_le_bytes());
    msg.extend_from_slice(buyer.pubkey().as_ref());
    msg.extend_from_slice(seller.pubkey().as_ref());
    msg.extend_from_slice(&[0u8; 16]); // market placeholder
    msg.extend_from_slice(&price.to_le_bytes());
    msg.extend_from_slice(&qty.to_le_bytes());
    msg.extend_from_slice(&timestamp.to_le_bytes());
    
    let signature = engine_identity.sign(&msg).to_bytes();
    let pubkey_bytes = engine_identity.pubkey().to_bytes();

    // 5. Construct Ed25519 Instruction Data
    let mut instruction_data = Vec::with_capacity(176 + msg.len());
    instruction_data.extend_from_slice(&[1, 0]); // num_signatures, padding
    instruction_data.extend_from_slice(&[112u8, 0]); // signature_offset
    instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
    instruction_data.extend_from_slice(&[48u8, 0]);  // pubkey_offset
    instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
    instruction_data.extend_from_slice(&[176u8, 0]); // message_offset
    instruction_data.extend_from_slice(&(msg.len() as u16).to_le_bytes());
    instruction_data.extend_from_slice(&[u16::MAX, u16::MAX].map(|x| x.to_le_bytes()).concat());
    
    instruction_data.extend_from_slice(&pubkey_bytes); // offset 48
    instruction_data.extend_from_slice(&signature);    // offset 112
    instruction_data.extend_from_slice(&msg);          // offset 176

    let verify_ix = Instruction {
        program_id: solana_sdk::ed25519_program::ID,
        accounts: vec![],
        data: instruction_data,
    };

    let settle_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(buyer_margin_pda, false),
            AccountMeta::new(seller_margin_pda, false),
            AccountMeta::new_readonly(sysvar::instructions::ID, false),
        ],
        data: perp_ix::SettleTrade { 
            trade_id, price, quantity: qty, buyer_nonce: 0, seller_nonce: 0 
        }.data(),
    };

    // 6. Atomic Execution
    let tx = Transaction::new_signed_with_payer(
        &[verify_ix, settle_ix],
        Some(&payer.pubkey()),
        &[&payer],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    
    // ASSERT SUCCESS
    assert!(result.is_ok(), "Settlement rejected: {:?}", result.err());
    
    // VERIFY STATE
    let buyer_acc_raw = svm.get_account(&buyer_margin_pda).unwrap();
    let buyer_acc = hybrid_perp_dex::state::MarginAccount::try_deserialize(&mut &buyer_acc_raw.data[..]).unwrap();
    assert_eq!(buyer_acc.nonce, 1);
}