//! RPC related to speculative execution.

use std::{str, sync::Arc};

use async_trait::async_trait;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use casper_types::{
    contract_messages::Messages, execution::ExecutionResultV2, BlockHash, Deploy, ProtocolVersion,
    Transaction,
};

use crate::node_interface::NodeInterface;

use super::{
    chain::BlockIdentifier,
    docs::{DocExample, DOCS_EXAMPLE_PROTOCOL_VERSION},
    Error, RpcWithParams,
};

static SPECULATIVE_EXEC_TXN_PARAMS: Lazy<SpeculativeExecTxnParams> =
    Lazy::new(|| SpeculativeExecTxnParams {
        block_identifier: Some(BlockIdentifier::Hash(*BlockHash::example())),
        transaction: Transaction::doc_example().clone(),
    });
static SPECULATIVE_EXEC_TXN_RESULT: Lazy<SpeculativeExecTxnResult> =
    Lazy::new(|| SpeculativeExecTxnResult {
        api_version: DOCS_EXAMPLE_PROTOCOL_VERSION,
        block_hash: *BlockHash::example(),
        execution_result: ExecutionResultV2::example().clone(),
        messages: Vec::new(),
    });
static SPECULATIVE_EXEC_PARAMS: Lazy<SpeculativeExecParams> = Lazy::new(|| SpeculativeExecParams {
    block_identifier: Some(BlockIdentifier::Hash(*BlockHash::example())),
    deploy: Deploy::doc_example().clone(),
});

/// Params for "speculative_exec_txn" RPC request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpeculativeExecTxnParams {
    /// Block hash on top of which to execute the transaction.
    pub block_identifier: Option<BlockIdentifier>,
    /// Transaction to execute.
    pub transaction: Transaction,
}

impl DocExample for SpeculativeExecTxnParams {
    fn doc_example() -> &'static Self {
        &SPECULATIVE_EXEC_TXN_PARAMS
    }
}

/// Result for "speculative_exec_txn" and "speculative_exec" RPC responses.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpeculativeExecTxnResult {
    /// The RPC API version.
    #[schemars(with = "String")]
    pub api_version: ProtocolVersion,
    /// Hash of the block on top of which the transaction was executed.
    pub block_hash: BlockHash,
    /// Result of the execution.
    pub execution_result: ExecutionResultV2,
    /// Messages emitted during execution.
    pub messages: Messages,
}

impl DocExample for SpeculativeExecTxnResult {
    fn doc_example() -> &'static Self {
        &SPECULATIVE_EXEC_TXN_RESULT
    }
}

/// "speculative_exec_txn" RPC
pub struct SpeculativeExecTxn {}

#[async_trait]
impl RpcWithParams for SpeculativeExecTxn {
    const METHOD: &'static str = "speculative_exec_txn";
    type RequestParams = SpeculativeExecTxnParams;
    type ResponseResult = SpeculativeExecTxnResult;

    async fn do_handle_request(
        node_interface: Arc<dyn NodeInterface>,
        api_version: ProtocolVersion,
        params: Self::RequestParams,
    ) -> Result<Self::ResponseResult, Error> {
        handle_request(
            node_interface,
            api_version,
            params.block_identifier,
            params.transaction,
        )
        .await
    }
}

/// Params for "speculative_exec" RPC request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpeculativeExecParams {
    /// Block hash on top of which to execute the deploy.
    pub block_identifier: Option<BlockIdentifier>,
    /// Deploy to execute.
    pub deploy: Deploy,
}

impl DocExample for SpeculativeExecParams {
    fn doc_example() -> &'static Self {
        &SPECULATIVE_EXEC_PARAMS
    }
}

/// "speculative_exec" RPC
pub struct SpeculativeExec {}

#[async_trait]
impl RpcWithParams for SpeculativeExec {
    const METHOD: &'static str = "speculative_exec";
    type RequestParams = SpeculativeExecParams;
    type ResponseResult = SpeculativeExecTxnResult;

    async fn do_handle_request(
        node_interface: Arc<dyn NodeInterface>,
        api_version: ProtocolVersion,
        params: Self::RequestParams,
    ) -> Result<Self::ResponseResult, Error> {
        handle_request(
            node_interface,
            api_version,
            params.block_identifier,
            Transaction::from(params.deploy),
        )
        .await
    }
}

async fn handle_request(
    _node_interface: Arc<dyn NodeInterface>,
    _api_version: ProtocolVersion,
    _maybe_block_id: Option<BlockIdentifier>,
    _transaction: Transaction,
) -> Result<SpeculativeExecTxnResult, Error> {
    todo!()
}
