use serde::{Deserialize, Serialize};
use risc0_zkvm::{Receipt};
use sp1_sdk::{SP1Proof, SP1VerifyingKey};
use crate::prover_type::ProverType;

#[derive(Clone, Deserialize, Serialize)]
pub struct Proof {
    pub receipt: Option<Receipt>,
    pub sp1proof: Option<SP1Proof>,
    pub vk: Option<SP1VerifyingKey>,
    pub prover_type: ProverType,
}
