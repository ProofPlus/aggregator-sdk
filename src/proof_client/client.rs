use ethers::abi::Abi;
use ethers::prelude::*;
use reqwest::Client;

use crate::prover_type::ProverType;
use crate::tx_sender::TxSender;
use crate::events::{TaskRequested, TaskFinalized};

use alloy_sol_types::{sol, SolInterface}; 
use risc0_zkvm::{default_executor, ExecutorEnv, compute_image_id, Receipt};

use ethers::utils::keccak256;
use sha2::{Sha256, Digest}; // Add this import for SHA256
use crate::models::Proof;

use alloy_primitives::{Bytes, U256, FixedBytes};
use serde_json::json; // Ensu

use std::sync::Arc;

sol! {
    interface ITaskManager {
        function requestTask(uint256 cycleCount) external returns (bytes32 taskId);
        function slash(bytes32 taskId, bytes32 publicInputsHash, bytes calldata proof) external;
    }
}

type TaskManagerContract = Contract<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>;

pub struct ProofPlusClient {
    pub tx_sender: TxSender,
    pub client: Client,
    pub prover_type: ProverType,
    contract: TaskManagerContract,
}

impl ProofPlusClient {
    pub fn new(
        chain_id: u64,
        rpc_url: &str,
        private_key: &str,
        contract_address: &str,
        prover_type: ProverType,
    ) -> Self {
        let tx_sender = TxSender::new(chain_id, rpc_url, private_key, contract_address).unwrap();
        let provider = Provider::<Http>::try_from(rpc_url).unwrap();
        let wallet: LocalWallet = private_key.parse::<LocalWallet>().unwrap().with_chain_id(chain_id);
        let client = SignerMiddleware::new(provider.clone(), wallet.clone());
        let contract_address_ethers = contract_address.parse::<ethers::types::H160>().unwrap();

        let abi: Abi = serde_json::from_slice(include_bytes!("../../artifacts/TaskManager.json")).unwrap();
        let contract = Contract::new(
            contract_address_ethers,
            abi,
            Arc::new(client.clone()),
        );

        Self {
            tx_sender: tx_sender,
            client: Client::new(),
            prover_type,
            contract: contract,
        }
    }

    pub async fn prove(self, elf: &[u8], inputs: &[u8]) -> Result<Receipt, Box<dyn std::error::Error>> {
        let cycle_count = self.compute_cycle_count(elf, inputs).await?;
        let calldata = self.create_request_task_call(cycle_count)?;

        self.tx_sender.send(calldata).await?;

        let task_requested_event = self.wait_for_task_requested_event().await?;
        self.handle_task_requested_event(task_requested_event, elf, inputs).await

    }

    async fn compute_cycle_count(&self, elf: &[u8], inputs: &[u8]) -> Result<U256, Box<dyn std::error::Error>> {
        let env = ExecutorEnv::builder().write_slice(inputs).build().unwrap();
        let session_info = default_executor().execute(env, elf).unwrap();
        Ok(U256::from(session_info.segments.iter().map(|seg| seg.cycles).sum::<u32>()))
    }

    fn create_request_task_call(&self, cycle_count: U256) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(ITaskManager::ITaskManagerCalls::requestTask(ITaskManager::requestTaskCall {
            cycleCount: cycle_count,
        })
        .abi_encode())
    }

    fn create_slash_call(&self, task_id: FixedBytes<32>, public_input_hash: FixedBytes<32>, proof: Bytes) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(ITaskManager::ITaskManagerCalls::slash(ITaskManager::slashCall {
            taskId: task_id,
            publicInputsHash: public_input_hash,
            proof: proof
        })
        .abi_encode())
    }

    async fn wait_for_task_requested_event(&self) -> Result<TaskRequested, Box<dyn std::error::Error>> {
        let filter = self.contract.event::<TaskRequested>().from_block(BlockNumber::Latest);
        let mut stream = filter.stream().await.unwrap();
        while let Some(Ok(log)) = stream.next().await {
            let TaskRequested { task_id: _, requester, prover: _, endpoint: _ } = log;
            if requester == self.tx_sender.wallet.address() {
                return Ok(log);
            }
        }
        Err("TaskRequested event not found".into())
    }

    async fn handle_task_requested_event(
        &self,
        event: TaskRequested,
        elf: &[u8],
        inputs: &[u8]
    ) -> Result<Receipt, Box<dyn std::error::Error>> {
        let image_id = compute_image_id(elf).unwrap().to_string();
        let payload = json!({
            "elf": elf,
            "inputs": inputs,
            "prover_type": self.prover_type,
            "requester_address": self.tx_sender.wallet.address(),
            "task_id": event.task_id,
            "image_id": image_id
        });

        let res = self.client.post(&event.endpoint).json(&payload).send().await?;

        if res.status().is_success() {
            println!("POST request sent successfully to {}", event.endpoint);
            let proof: Proof = res.json().await?;
            if proof.prover_type == ProverType::RiscZero {
                if let Some(receipt) = &proof.receipt {
                    let public_input_hash = self.compute_public_input_hash(&receipt.journal.bytes)?;
                    let proof_hash = keccak256(&receipt.inner.groth16()?.seal);
                    self.wait_for_task_finalized_event(&event.task_id, &public_input_hash, &proof_hash).await?;
                
                    return Ok(receipt.clone());
                }
            }
        } else {
            return Err("Failed to send POST request".into());
        }
        
        return Err("Failed to handle task requested event".into());
    }

    fn compute_public_input_hash(&self, journal: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut hasher = Sha256::new();
        hasher.update(journal);
        Ok(hasher.finalize().to_vec())
    }

    async fn wait_for_task_finalized_event(
        &self,
        task_id: &H256,
        public_input_hash: &[u8],
        proof_hash: &[u8]
    ) -> Result<(), Box<dyn std::error::Error>> {
        let filter = self.contract.event::<TaskFinalized>().from_block(BlockNumber::Latest);
        let mut stream = filter.stream().await.unwrap();

        while let Some(Ok(log)) = stream.next().await {
            let TaskFinalized { task_id: task_id_finalized, image_id: _, public_input_hash: public_input_hash_finalized, proof_hash: proof_hash_finalized } = log;
            if task_id_finalized == *task_id {
                if public_input_hash_finalized != public_input_hash || proof_hash_finalized != proof_hash {
                    println!("Task finalized event received with different public input hash or proof hash.");
                    // TODO: call slash method
                }
                break;
            }
        }
        Ok(())
    }

}
