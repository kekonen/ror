//! Rorschach Solana Client
//!
//! Submits Groth16 proofs to Solana for verification

use borsh::BorshSerialize;
use clap::{Parser, Subcommand};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "rorschach-client")]
#[command(about = "Submit Rorschach proofs to Solana", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Solana RPC URL
    #[arg(long, default_value = "http://127.0.0.1:8899")]
    rpc_url: String,

    /// Path to keypair file
    #[arg(long, default_value = "~/.config/solana/id.json")]
    keypair: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the verifying key account
    InitVk {
        /// Path to verifying key file
        #[arg(long)]
        vk_file: PathBuf,

        /// Program ID
        #[arg(long)]
        program_id: String,
    },

    /// Submit a proof for verification
    Verify {
        /// Path to proof file
        #[arg(long)]
        proof: PathBuf,

        /// Path to public inputs file
        #[arg(long)]
        public_inputs: PathBuf,

        /// Program ID
        #[arg(long)]
        program_id: String,

        /// Verifying key account address
        #[arg(long)]
        vk_account: String,
    },

    /// Deploy the program
    Deploy {
        /// Path to compiled program .so file
        #[arg(long)]
        program_path: PathBuf,
    },
}

/// Instruction data matching the program
#[derive(BorshSerialize)]
enum RorschachInstruction {
    VerifyProof {
        proof: Vec<u8>,
        public_inputs: Vec<u8>,
    },
    InitializeVerifyingKey {
        verifying_key: Vec<u8>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let rpc_client = RpcClient::new_with_commitment(
        cli.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Load keypair
    let keypair_path = shellexpand::tilde(&cli.keypair).to_string();
    let keypair_data = fs::read_to_string(&keypair_path)?;
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)?;
    let keypair = Keypair::from_bytes(&keypair_bytes)?;

    println!("Using wallet: {}", keypair.pubkey());
    println!("RPC URL: {}", cli.rpc_url);

    match cli.command {
        Commands::InitVk {
            vk_file,
            program_id,
        } => {
            println!("\n=== Initializing Verifying Key ===\n");
            init_verifying_key(&rpc_client, &keypair, vk_file, &program_id)?;
        }

        Commands::Verify {
            proof,
            public_inputs,
            program_id,
            vk_account,
        } => {
            println!("\n=== Submitting Proof for Verification ===\n");
            verify_proof(
                &rpc_client,
                &keypair,
                proof,
                public_inputs,
                &program_id,
                &vk_account,
            )?;
        }

        Commands::Deploy { program_path } => {
            println!("\n=== Deploying Program ===\n");
            deploy_program(&rpc_client, &keypair, program_path)?;
        }
    }

    Ok(())
}

fn init_verifying_key(
    client: &RpcClient,
    payer: &Keypair,
    vk_file: PathBuf,
    program_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id = Pubkey::from_str(program_id)?;

    // Load verifying key
    let vk_data = fs::read(&vk_file)?;
    println!("Loaded verifying key: {} bytes", vk_data.len());

    // Create verifying key account (deterministic PDA)
    let (vk_account, _bump) = Pubkey::find_program_address(&[b"verifying-key"], &program_id);
    println!("Verifying key account: {}", vk_account);

    // Calculate required space
    let space = 8 + 32 + 4 + vk_data.len(); // discriminator + pubkey + vec len + data
    let rent = client.get_minimum_balance_for_rent_exemption(space)?;

    println!("Creating account with {} bytes (rent: {} lamports)", space, rent);

    // Create account instruction
    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &vk_account,
        rent,
        space as u64,
        &program_id,
    );

    // Initialize verifying key instruction
    let init_vk_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vk_account, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: RorschachInstruction::InitializeVerifyingKey {
            verifying_key: vk_data,
        }
        .try_to_vec()?,
    };

    // Send transaction
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix, init_vk_ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction)?;
    println!("\n✓ Verifying key initialized!");
    println!("Signature: {}", signature);
    println!("VK Account: {}", vk_account);

    Ok(())
}

fn verify_proof(
    client: &RpcClient,
    payer: &Keypair,
    proof_file: PathBuf,
    public_inputs_file: PathBuf,
    program_id: &str,
    vk_account: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let program_id = Pubkey::from_str(program_id)?;
    let vk_account = Pubkey::from_str(vk_account)?;

    // Load proof and public inputs
    let proof = fs::read(&proof_file)?;
    let public_inputs = fs::read(&public_inputs_file)?;

    println!("Proof: {} bytes", proof.len());
    println!("Public inputs: {} bytes", public_inputs.len());

    // Create verification record account (PDA based on user + timestamp)
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let (record_account, _bump) = Pubkey::find_program_address(
        &[b"verification", payer.pubkey().as_ref(), &timestamp.to_le_bytes()],
        &program_id,
    );

    println!("Verification record: {}", record_account);

    // Calculate space for verification record
    let space = 8 + 32 + 8 + 32 + 1; // discriminator + pubkey + i64 + hash + bool
    let rent = client.get_minimum_balance_for_rent_exemption(space)?;

    // Create record account
    let create_record_ix = system_instruction::create_account(
        &payer.pubkey(),
        &record_account,
        rent,
        space as u64,
        &program_id,
    );

    // Verify proof instruction
    let verify_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(record_account, false),
            AccountMeta::new_readonly(payer.pubkey(), true),
            AccountMeta::new_readonly(vk_account, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(solana_sdk::sysvar::instructions::id(), false),
        ],
        data: RorschachInstruction::VerifyProof {
            proof,
            public_inputs,
        }
        .try_to_vec()?,
    };

    // Send transaction
    println!("\nSubmitting proof to Solana...");
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[create_record_ix, verify_ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = client.send_and_confirm_transaction(&transaction)?;

    println!("\n✓ Proof verified successfully!");
    println!("Signature: {}", signature);
    println!("Record: {}", record_account);
    println!("\nView on explorer:");
    println!("https://explorer.solana.com/tx/{}?cluster=custom&customUrl={}", signature, client.url());

    Ok(())
}

fn deploy_program(
    _client: &RpcClient,
    _payer: &Keypair,
    _program_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Use 'solana program deploy' command instead:");
    println!("  solana program deploy target/deploy/rorschach_solana.so");
    Ok(())
}
