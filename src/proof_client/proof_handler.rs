use reqwest::Client;
use ethers::utils::{keccak256, hex};
use sha2::{Sha256, Digest}; // Add this import for SHA256
use crate::models::Proof;
use crate::prover_type::ProverType;
use ethers::prelude::*; // Ensure proper ethers imports
use diesel::prelude::*;
use crate::orm::{establish_connection, FinalizedTask};
use std::time::Duration;
use tokio::time::sleep;
use serde_json::json; // Ensure this import for json macro

pub async fn send_proof_request(
    client: &Client,
    endpoint: &str,
    // signature: &str,
    elf: &[u8],
    inputs: &[u8],
    prover_type: &ProverType,
    requester_address: &H160,
    task_id: &str, // Added task_id
) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the JSON payload
    let payload = json!({
        // "signature": signature,
        "elf": elf,
        "inputs": inputs,
        "prover_type": prover_type,
        "requester_address": requester_address,
    });

    // Send the POST request
    let res = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    if res.status().is_success() {
        println!("POST request sent successfully to {}", endpoint);

        // Parse the response into the Proof struct
        let proof: Proof = res.json().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        // Check if prover_type is RiscZero and compute the hashes
        if proof.prover_type == ProverType::RiscZero {
            if let Some(receipt) = &proof.receipt {
                // Compute the public input hash (SHA256 of the journal)
                let mut hasher = Sha256::new();
                hasher.update(&receipt.journal);
                let public_input_hash = hasher.finalize();

                // Compute the proof hash (Keccak256 of the seal)
                let proof_hash = keccak256(&receipt.inner.groth16()?.seal);

                // Establish database connection
                let mut conn = establish_connection();

                // Retry mechanism to fetch the task from the database
                for _ in 0..5 {
                    let results: Vec<FinalizedTask> = crate::orm::finalized_tasks::table
                        .filter(crate::orm::finalized_tasks::task_id.eq(task_id))
                        .load(&mut conn)
                        .unwrap_or_else(|_| vec![]);

                    if !results.is_empty() {
                        let db_public_input_hash = &results[0].public_input_hash;
                        let db_proof_hash = &results[0].proof_hash;

                        // Validate hashes
                        if hex::encode(public_input_hash) == *db_public_input_hash {
                            println!("Public input hash matches the database value.");
                        } else {
                            println!("Public input hash does not match the database value.");
                        }

                        if hex::encode(proof_hash) == *db_proof_hash {
                            println!("Proof hash matches the database value.");
                        } else {
                            println!("Proof hash does not match the database value.");
                        }

                        return Ok(());
                    }

                    // Wait before retrying
                    sleep(Duration::from_secs(2)).await;
                }

                println!("Failed to fetch matching task from the database after retries.");
            }
        }
    } else {
        println!("Failed to send POST request: {}", res.status());
    }
    Ok(())
}