//! RPCs related to the foo.

use async_trait::async_trait;
use casper_json_rpc::Error;
use casper_types::{
    bytesrepr::{self, FromBytes},
    BlockHash, ProtocolVersion,
};
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{components::rpc_server::ReactorEventT, effect::EffectBuilder};

use super::{docs::DocExample, RpcWithParams};

static FOO_PARAMS: Lazy<FooParams> = Lazy::new(|| FooParams {
    kind: RequestKind::GetBlockBody,
    payload: vec![],
});
static FOO_RESULT: Lazy<FooResult> = Lazy::new(|| FooResult {
    kind: ResultKind::Ok,
});

/// Kind of the request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum RequestKind {
    /// Returns the `BlockBody`. Payload is expected to be the serialized body hash.
    GetBlockBody = 1,
    /// Returns the `BlockHeader`. Payload is expected to be the serialized hash of the block.
    GetBlockHeader = 2,
}

/// Params for "foo" RPC request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FooParams {
    /// The request kind.
    pub kind: RequestKind,
    /// The request payload.
    pub payload: Vec<u8>,
}

impl DocExample for FooParams {
    fn doc_example() -> &'static Self {
        &FOO_PARAMS
    }
}

/// Kind of the response.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ResultKind {
    /// Request was executed correctly.
    Ok = 1,
    /// There was an error executing request.
    Error = 2,
}

/// Result for "foo" RPC response.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FooResult {
    kind: ResultKind,
}

impl DocExample for FooResult {
    fn doc_example() -> &'static Self {
        &FOO_RESULT
    }
}

/// "foo" RPC.
pub struct Foo {}

#[async_trait]
impl RpcWithParams for Foo {
    const METHOD: &'static str = "foo";

    type RequestParams = FooParams;
    type ResponseResult = FooResult;

    async fn do_handle_request<REv: ReactorEventT>(
        _effect_builder: EffectBuilder<REv>,
        _api_version: ProtocolVersion,
        params: Self::RequestParams,
    ) -> Result<Self::ResponseResult, Error> {
        let kind = params.kind;
        match kind {
            RequestKind::GetBlockBody => (),
            RequestKind::GetBlockHeader => {
                let maybe_block_hash: Result<BlockHash, bytesrepr::Error> =
                    BlockHash::from_bytes(params.payload.as_slice());

                //.map_err(|bytesrepr_error| Error::new(-32602, bytesrepr_error.to_string()))?;
            }
        };
        Ok(FooResult {
            kind: ResultKind::Error,
        })
    }
}
