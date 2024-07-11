use ethers::prelude::*;

#[derive(Debug, Clone, EthEvent)]
pub struct TaskRequested {
    #[ethevent(indexed)]
    pub task_id: H256,
    #[ethevent(indexed)]
    pub requester: H160,
    #[ethevent(indexed)]
    pub prover: H160,
    pub endpoint: String,
}

#[derive(Debug, Clone, EthEvent)]
pub struct TaskFinalized {
    #[ethevent(indexed)]
    pub task_id: H256,
    #[ethevent(indexed)]
    pub image_id: H256,
    pub public_input_hash: Vec<u8>,
    pub proof_hash: Vec<u8>,
}
