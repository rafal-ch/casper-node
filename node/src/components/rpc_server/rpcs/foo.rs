//! RPCs related to the foo.

use async_trait::async_trait;
use casper_json_rpc::Error;
use casper_types::ProtocolVersion;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{components::rpc_server::ReactorEventT, effect::EffectBuilder};

use super::{docs::DocExample, RpcWithParams};

static FOO_PARAMS: Lazy<FooParams> = Lazy::new(|| FooParams {
    kind: Kind::GetBlockBody,
    payload: vec![],
});
static FOO_RESULT: Lazy<FooResult> = Lazy::new(|| FooResult { number: 321 });

/// Kind of the request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum Kind {
    /// Returns the BlockBody. Payload is expected to be the serialized hash of the block.
    GetBlockBody = 1,
}

/// Params for "foo" RPC request.
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FooParams {
    /// The request kind.
    pub kind: Kind,
    /// The request payload.
    pub payload: Vec<u8>,
}

impl DocExample for FooParams {
    fn doc_example() -> &'static Self {
        &FOO_PARAMS
    }
}

/// Result for "foo" RPC response.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FooResult {
    /// The other number.
    pub number: usize,
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
            Kind::GetBlockBody => (),
        };
        Ok(FooResult { number: 888 })
    }
}
