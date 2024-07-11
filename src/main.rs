use dotenv::dotenv;
use std::env;
// use std::sync::Arc;
use tracing::Level;
use proofplus_client::{ProofPlusClient, ProverType};
use ethers::utils::hex;

use ethers::abi::Token;
use std::io::Write;

use alloy_primitives::FixedBytes;

// use risc0_ethereum_contracts::groth16::Seal;
use anyhow::Context;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    let chain_id: u64 = env::var("CHAIN_ID")?.parse()?;
    let rpc_url = env::var("RPC_URL")?;
    let private_key = env::var("PRIVATE_KEY")?;
    let contract_address = env::var("CONTRACT_ADDRESS")?;

    // Create the ProofPlusClient as an Arc
    let client = ProofPlusClient::new(
        chain_id,
        &rpc_url,
        &private_key,
        &contract_address,
        ProverType::RiscZero
    );

    // Call the prove method
    client.prove(&[], &[]).await?;

    // Sample values for journal, post_state_digest, and seal (assuming these are defined somewhere in your actual code)
    let journal: Vec<u8> = vec![]; // Replace with actual data
    let post_state_digest: FixedBytes<32> = FixedBytes([0u8; 32]); // Replace with actual data
    let seal: Vec<u8> = vec![]; // Replace with actual data

    let calldata = vec![
        Token::Bytes(journal),
        Token::FixedBytes(post_state_digest.to_vec()),
        Token::Bytes(seal),
    ];
    let output = hex::encode(ethers::abi::encode(&calldata));

    // Forge test FFI calls expect hex encoded bytes sent to stdout
    print!("{output}");
    std::io::stdout()
        .flush()
        .context("failed to flush stdout buffer")?;

    Ok(())
}
