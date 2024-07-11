use ethers::utils::hex;
use ethers::prelude::*; // Ensure proper ethers imports
use crate::proof_client::proof_handler::send_proof_request;
use crate::orm::{establish_connection, create_finalized_task};
use crate::prover_type::ProverType;
use std::sync::Arc;
use reqwest::Client;

pub async fn handle_task_requested_event(
    task_id: H256,
    requester: H160,
    prover: H160,
    endpoint: String,
    client: &Arc<Client>,
    // client: &Arc<ProofPlusClient>,
    elf: &Arc<Vec<u8>>,
    inputs: &Arc<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("TaskRequested event received: task_id: {:?}, requester: {:?}, prover: {:?}, endpoint: {:?}", task_id, requester, prover, endpoint);

    // let requester_signature = client.wallet.sign_message(requester.as_bytes()).await?;
    let prover_type = ProverType::RiscZero;

    send_proof_request(
        &client,
        &endpoint,
        // &requester_signature.to_string(),
        elf,
        inputs,
        &prover_type,
        &requester,
        &task_id.to_string(), // Pass task_id
    ).await?;

    Ok(())
}

pub async fn handle_task_finalized_event(
    task_id: H256,
    image_id: H256,
    public_input_hash: Vec<u8>,
    proof_hash: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("TaskFinalized event received: task_id: {:?}, image_id: {:?}, public_input_hash: {:?}, proof_hash: {:?}", task_id, image_id, public_input_hash, proof_hash);

    // Store the finalized task to the local database
    let mut conn = establish_connection();
    create_finalized_task(
        &mut conn,
        &task_id.to_string(),
        &image_id.to_string(),
        &hex::encode(public_input_hash),
        &hex::encode(proof_hash),
    );

    Ok(())
}