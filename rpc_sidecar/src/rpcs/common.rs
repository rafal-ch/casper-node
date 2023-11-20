use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::rpcs::Error;
use casper_types::{
    AvailableBlockRange, Block, BlockSignatures, ExecutionInfo, FinalizedApprovals, SignedBlock,
    Transaction, TransactionHash,
};

use crate::NodeClient;

use super::{chain::BlockIdentifier, ErrorCode};

pub(super) static MERKLE_PROOF: Lazy<String> = Lazy::new(|| {
    String::from(
        "01000000006ef2e0949ac76e55812421f755abe129b6244fe7168b77f47a72536147614625016ef2e0949ac76e\
        55812421f755abe129b6244fe7168b77f47a72536147614625000000003529cde5c621f857f75f3810611eb4af3\
        f998caaa9d4a3413cf799f99c67db0307010000006ef2e0949ac76e55812421f755abe129b6244fe7168b77f47a\
        7253614761462501010102000000006e06000000000074769d28aac597a36a03a932d4b43e4f10bf0403ee5c41d\
        d035102553f5773631200b9e173e8f05361b681513c14e25e3138639eb03232581db7557c9e8dbbc83ce9450022\
        6a9a7fe4f2b7b88d5103a4fc7400f02bf89c860c9ccdd56951a2afe9be0e0267006d820fb5676eb2960e15722f7\
        725f3f8f41030078f8b2e44bf0dc03f71b176d6e800dc5ae9805068c5be6da1a90b2528ee85db0609cc0fb4bd60\
        bbd559f497a98b67f500e1e3e846592f4918234647fca39830b7e1e6ad6f5b7a99b39af823d82ba1873d0000030\
        00000010186ff500f287e9b53f823ae1582b1fa429dfede28015125fd233a31ca04d5012002015cc42669a55467\
        a1fdf49750772bfc1aed59b9b085558eb81510e9b015a7c83b0301e3cf4a34b1db6bfa58808b686cb8fe21ebe0c\
        1bcbcee522649d2b135fe510fe3")
});

/// An enum to be used as the `data` field of a JSON-RPC error response.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields, untagged)]
pub enum ErrorData {
    /// The requested block of state root hash is not available on this node.
    MissingBlockOrStateRoot {
        /// Additional info.
        message: String,
        /// The height range (inclusive) of fully available blocks.
        available_block_range: AvailableBlockRange,
    },
}

pub async fn get_signed_block(
    node_client: &dyn NodeClient,
    identifier: Option<BlockIdentifier>,
) -> Result<SignedBlock, Error> {
    let hash = match identifier {
        Some(BlockIdentifier::Hash(hash)) => hash,
        Some(BlockIdentifier::Height(height)) => node_client
            .read_block_hash_from_height(height)
            .await
            .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
            .ok_or_else(|| Error::new(ErrorCode::NoSuchBlock, "no block at requested height"))?,
        None => *node_client
            .read_highest_completed_block_info()
            .await
            .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
            .ok_or_else(|| Error::new(ErrorCode::NoSuchBlock, "no coompleted block available"))?
            .block_hash(),
    };

    let header = node_client
        .read_block_header(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
        .ok_or_else(|| {
            Error::new(
                ErrorCode::NoSuchBlock,
                format!("block header not found for {hash}"),
            )
        })?;
    let body = node_client
        .read_block_body(*header.body_hash())
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
        .ok_or_else(|| {
            Error::new(
                ErrorCode::NoSuchBlock,
                format!("block body not found for {hash}"),
            )
        })?;
    let signatures = node_client
        .read_block_signatures(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
        .unwrap_or_else(|| BlockSignatures::new(hash, header.era_id()));
    let block = Block::new_from_header_and_body(header, body).unwrap();

    if signatures.is_verified().is_err() {
        return Err(Error::new(
            ErrorCode::InvalidBlock,
            format!("block {} could not be verified", hash),
        ));
    };

    Ok(SignedBlock::new(block, signatures))
}

pub async fn get_transaction_with_approvals(
    node_client: &dyn NodeClient,
    hash: TransactionHash,
) -> Result<(Transaction, Option<FinalizedApprovals>), Error> {
    let txn = node_client
        .read_transaction(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
        .ok_or_else(|| {
            Error::new(
                ErrorCode::NoSuchTransaction,
                format!("transaction not found for {hash}"),
            )
        })?;
    let approvals = node_client
        .read_finalized_approvals(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?;
    Ok((txn, approvals))
}

pub async fn get_transaction_execution_info(
    node_client: &dyn NodeClient,
    hash: TransactionHash,
) -> Result<Option<ExecutionInfo>, Error> {
    let Some(block_hash_and_height) = node_client
        .read_transaction_block_info(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?
        else { return Ok(None) };
    let execution_result = node_client
        .read_execution_result(hash)
        .await
        .map_err(|err| Error::new(ErrorCode::QueryFailed, err.to_string()))?;

    Ok(Some(ExecutionInfo {
        block_hash: *block_hash_and_height.block_hash(),
        block_height: block_hash_and_height.block_height(),
        execution_result,
    }))
}
