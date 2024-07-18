// lib.rs
pub mod prover_type;
pub mod tx_sender;
pub mod proof_client;
pub mod events;
pub mod models;

pub use prover_type::ProverType;
pub use tx_sender::TxSender;
pub use proof_client::ProofPlusClient;
pub use events::{TaskRequested, TaskFinalized};
pub use models::Proof;
