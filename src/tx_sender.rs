

use ethers::signers::LocalWallet;

use ethers::prelude::*;

/// Wrapper of a `SignerMiddleware` client to send transactions to the given
/// contract's `Address`.
pub struct TxSender {
    pub client: SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>,
    pub wallet: LocalWallet,
    pub provider: Provider<Http>,
    pub contract: ethers::types::H160,
    pub chain_id: u64,
}

impl TxSender {
    /// Creates a new `TxSender`.
    pub fn new(
        chain_id: u64,
        rpc_url: &str,
        private_key: &str,
        contract: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        let client = SignerMiddleware::new(provider.clone(), wallet.clone());
        let contract = contract.parse::<ethers::types::H160>()?;

        Ok(TxSender {
            provider,
            wallet,
            chain_id,
            client,
            contract,
        })
    }

    /// Send a transaction with the given calldata.
    pub async fn send(
        &self,
        calldata: Vec<u8>,
    ) -> Result<Option<TransactionReceipt>, Box<dyn std::error::Error>> {
        let tx = TransactionRequest::new()
            .chain_id(self.chain_id)
            .to(self.contract)
            .from(self.client.address())
            .data(calldata);

        tracing::info!("Transaction request: {:?}", &tx);

        let tx = self.client.send_transaction(tx, None).await?.await?;

        tracing::info!("Transaction receipt: {:?}", &tx);

        Ok(tx)
    }
}
