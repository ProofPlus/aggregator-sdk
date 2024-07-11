use ethers::abi::Abi;
use ethers::prelude::*;
// use ethers::signers::LocalWallet;
use reqwest::Client;
use std::sync::Arc;

use crate::prover_type::ProverType;
use crate::tx_sender::TxSender;
use crate::events::{TaskRequested, TaskFinalized};
use crate::proof_client::{handle_task_requested_event, handle_task_finalized_event};

use alloy_sol_types::{sol, SolInterface}; 

sol! {
    interface ITaskManager {
        function requestTask() external;
    }
}

pub struct ProofPlusClient {
    pub tx_sender: TxSender,
    pub client: Client,
    pub prover_type: ProverType,
    // pub requester_address: ethers::types::H160,
}
impl ProofPlusClient {
    pub fn new(
        chain_id: u64,
        rpc_url: &str,
        private_key: &str,
        contract: &str,
        prover_type: ProverType,
        // requester_address: ethers::types::H160,
    ) -> Self {

        // tx_sender = TxSender::new(chain_id, rpc_url, private_key, contract)?;
        // client = Client::new();

        Self {
            tx_sender: TxSender::new(chain_id, rpc_url, private_key, contract).unwrap(),
            client: Client::new(),
            prover_type,
            // requester_address,
        }
    }

    pub async fn prove(self, elf: &[u8], inputs: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let calldata = ITaskManager::ITaskManagerCalls::requestTask(ITaskManager::requestTaskCall {}).abi_encode();

        // Send transaction
        self.tx_sender.send(calldata).await?;

        // Fetch ABI and create contract instance
        let abi: Abi = serde_json::from_slice(include_bytes!("../../artifacts/TaskManager.json"))?;
        let contract = Contract::new(
            self.tx_sender.contract.clone(),
            abi,
            Arc::new(self.tx_sender.client.clone()),
        );

        let elf_vec = Arc::new(elf.to_vec());
        let inputs_vec = Arc::new(inputs.to_vec());

        // Start listening for the TaskRequested event in a separate async block
        {
            let client_arc = Arc::new(self.client.clone());
            let elf_vec = Arc::clone(&elf_vec);
            let inputs_vec = Arc::clone(&inputs_vec);
            let contract_clone = contract.clone();
            tokio::spawn(async move {
                let filter_requested = contract_clone
                    .event::<TaskRequested>()
                    .from_block(BlockNumber::Latest);
                let mut stream_requested = filter_requested.stream().await.unwrap();

                while let Some(Ok(log)) = stream_requested.next().await {
                    let TaskRequested { task_id, requester, prover, endpoint } = log;
                    if let Err(e) = handle_task_requested_event(task_id, requester, prover, endpoint, &client_arc, &elf_vec, &inputs_vec).await {
                        eprintln!("Error handling TaskRequested event: {:?}", e);
                    }
                }
            });
        }

        // Start listening for the TaskFinalized event in a separate async block
        
        tokio::spawn(async move {
            let filter_finalized = contract
                .event::<TaskFinalized>()
                .from_block(BlockNumber::Latest);
            let mut stream_finalized = filter_finalized.stream().await.unwrap();

            while let Some(Ok(log)) = stream_finalized.next().await {
                let TaskFinalized { task_id, image_id, public_input_hash, proof_hash } = log;
                if let Err(e) = handle_task_finalized_event(task_id, image_id, public_input_hash, proof_hash).await {
                    eprintln!("Error handling TaskFinalized event: {:?}", e);
                }
            }
        });
        

        Ok(())
    }
}