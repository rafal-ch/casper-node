use std::fmt::{self, Display, Formatter};

use derive_more::From;

use casper_types::EraId;

use crate::{
    effect::requests::BlockAccumulatorRequest,
    types::{ApprovalsHashes, Block, FinalitySignature, NodeId},
};

#[derive(Debug, From)]
pub(crate) enum Event {
    #[from]
    Request(BlockAccumulatorRequest),
    ReceivedBlock {
        block: Box<Block>,
        sender: NodeId,
    },
    ReceivedApprovalsHashes {
        approvals_hashes: Box<ApprovalsHashes>,
        sender: NodeId,
    },
    ReceivedFinalitySignature {
        finality_signature: Box<FinalitySignature>,
        sender: NodeId,
    },
    UpdatedValidatorMatrix {
        era_id: EraId,
    },
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Event::Request(BlockAccumulatorRequest::GetPeersForBlock { block_hash, .. }) => {
                write!(
                    f,
                    "block accumulator peers request for block: {}",
                    block_hash
                )
            }
            Event::ReceivedBlock { block, sender } => {
                write!(f, "received {} from {}", block, sender)
            }
            Event::ReceivedFinalitySignature {
                finality_signature,
                sender,
            } => {
                write!(f, "received {} from {}", finality_signature, sender)
            }
            Event::UpdatedValidatorMatrix { era_id } => {
                write!(f, "validator matrix update for era {}", era_id)
            }
            Event::ReceivedApprovalsHashes {
                approvals_hashes: _,
                sender,
            } => {
                write!(f, "approvals hashes update from {}", sender)
            }
        }
    }
}