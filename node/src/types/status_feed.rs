// TODO - remove once schemars stops causing warning.
#![allow(clippy::field_reassign_with_default)]

use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use datasize::DataSize;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use casper_hashing::Digest;
use casper_types::{EraId, ProtocolVersion, PublicKey, TimeDiff, Timestamp};

use crate::{
    components::{
        chainspec_loader::NextUpgrade,
        rpc_server::rpcs::docs::{DocExample, DOCS_EXAMPLE_PROTOCOL_VERSION},
    },
    types::{ActivationPoint, Block, BlockHash, NodeId, PeersMap},
};

static CHAINSPEC_INFO: Lazy<ChainspecInfo> = Lazy::new(|| {
    let next_upgrade = NextUpgrade::new(
        ActivationPoint::EraId(EraId::from(42)),
        ProtocolVersion::from_parts(2, 0, 1),
    );
    ChainspecInfo {
        name: String::from("casper-example"),
        next_upgrade: Some(next_upgrade),
    }
});

static GET_STATUS_RESULT: Lazy<GetStatusResult> = Lazy::new(|| {
    let node_id = NodeId::doc_example();
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 54321);
    let mut peers = BTreeMap::new();
    peers.insert(*node_id, socket_addr.to_string());
    let status_feed = StatusFeed {
        last_added_block: Some(Block::doc_example().clone()),
        peers,
        chainspec_info: ChainspecInfo::doc_example().clone(),
        our_public_signing_key: Some(PublicKey::doc_example().clone()),
        round_length: Some(TimeDiff::from(1 << 16)),
        version: crate::VERSION_STRING.as_str(),
        node_uptime: Duration::from_secs(13),
        node_state: NodeState::new_participating(),
    };
    GetStatusResult::new(status_feed, DOCS_EXAMPLE_PROTOCOL_VERSION)
});

/// Summary information from the chainspec.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainspecInfo {
    /// Name of the network.
    name: String,
    next_upgrade: Option<NextUpgrade>,
}

impl DocExample for ChainspecInfo {
    fn doc_example() -> &'static Self {
        &*CHAINSPEC_INFO
    }
}

impl ChainspecInfo {
    pub(crate) fn new(chainspec_network_name: String, next_upgrade: Option<NextUpgrade>) -> Self {
        ChainspecInfo {
            name: chainspec_network_name,
            next_upgrade,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, DataSize, Debug, Serialize, Deserialize, JsonSchema)]
pub enum SyncStatus {
    /// Node is fully synced.
    FullySynced,
    /// Node is still running the sync to genesis process in the background.
    SyncInProgress {
        full_block_height: Option<u64>,
        destination_block_height: Option<u64>,
        percent: Option<u64>,
    },
}

impl SyncStatus {
    pub fn new() -> Self {
        Self::SyncInProgress {
            full_block_height: None,
            destination_block_height: None,
            percent: None,
        }
    }

    fn with_progress(full_block_height: u64, destination_block_height: Option<u64>) -> Self {
        let percent = match destination_block_height {
            Some(destination_block_height) if destination_block_height > 0 => {
                Some(full_block_height * 100 / destination_block_height)
            }
            None | Some(_) => None,
        };
        Self::SyncInProgress {
            full_block_height: Some(full_block_height),
            destination_block_height,
            percent,
        }
    }

    fn with_destination(self, new_destination_block_height: u64) -> Self {
        match self {
            SyncStatus::FullySynced => self,
            SyncStatus::SyncInProgress {
                full_block_height,
                destination_block_height: _,
                percent: _,
            } => SyncStatus::SyncInProgress {
                full_block_height,
                destination_block_height: Some(new_destination_block_height),
                percent: if let Some(full_block_height) = full_block_height {
                    if new_destination_block_height > 0 {
                        Some(full_block_height * 100 / new_destination_block_height)
                    } else {
                        None
                    }
                } else {
                    None
                },
            },
        }
    }
}

/// The various possible states of operation for the node.
#[derive(Clone, Copy, PartialEq, Eq, DataSize, Debug, Serialize, Deserialize, JsonSchema)]
pub enum NodeState {
    /// The node is currently in the fast syncing state.
    FastSyncing,
    /// The node is currently participating, and optionally running the sync to genesis in the background
    Participating(SyncStatus),
}

impl NodeState {
    pub(crate) fn new_participating() -> Self {
        Self::Participating(SyncStatus::new())
    }

    pub(crate) fn with_updated_progress(&self, new_full_block_height: u64) -> Self {
        match self {
            NodeState::Participating(SyncStatus::SyncInProgress {
                full_block_height,
                destination_block_height,
                percent: _,
            }) => match full_block_height {
                Some(current_full_block_height)
                    if new_full_block_height > *current_full_block_height =>
                {
                    NodeState::Participating(SyncStatus::with_progress(
                        new_full_block_height,
                        *destination_block_height,
                    ))
                }
                None => NodeState::Participating(SyncStatus::with_progress(
                    new_full_block_height,
                    *destination_block_height,
                )),
                Some(_) => *self,
            },
            NodeState::FastSyncing | NodeState::Participating(_) => *self,
        }
    }

    pub(crate) fn new_syncing_finished() -> Self {
        Self::Participating(SyncStatus::FullySynced)
    }

    pub(crate) fn with_updated_destination_height(&self, destination_block_height: u64) -> Self {
        match self {
            NodeState::FastSyncing | NodeState::Participating(SyncStatus::FullySynced) => *self,
            NodeState::Participating(SyncStatus::SyncInProgress {
                full_block_height,
                percent, // TODO[RC]: Should ignore here
                destination_block_height: _,
            }) => NodeState::Participating(SyncStatus::SyncInProgress {
                full_block_height: *full_block_height,
                destination_block_height: Some(destination_block_height),
                percent: *percent,
            }),
        }
    }
}

/// Data feed for client "info_get_status" endpoint.
#[derive(Debug, Serialize)]
pub struct StatusFeed {
    /// The last block added to the chain.
    pub last_added_block: Option<Block>,
    /// The peer nodes which are connected to this node.
    pub peers: BTreeMap<NodeId, String>,
    /// The chainspec info for this node.
    pub chainspec_info: ChainspecInfo,
    /// Our public signing key.
    pub our_public_signing_key: Option<PublicKey>,
    /// The next round length if this node is a validator.
    pub round_length: Option<TimeDiff>,
    /// The compiled node version.
    pub version: &'static str,
    /// Time that passed since the node has started.
    pub node_uptime: Duration,
    /// The current state of node.
    pub node_state: NodeState,
}

impl StatusFeed {
    pub(crate) fn new(
        last_added_block: Option<Block>,
        peers: BTreeMap<NodeId, String>,
        chainspec_info: ChainspecInfo,
        consensus_status: Option<(PublicKey, Option<TimeDiff>)>,
        node_uptime: Duration,
        node_state: NodeState,
    ) -> Self {
        let (our_public_signing_key, round_length) = match consensus_status {
            Some((public_key, round_length)) => (Some(public_key), round_length),
            None => (None, None),
        };
        StatusFeed {
            last_added_block,
            peers,
            chainspec_info,
            our_public_signing_key,
            round_length,
            version: crate::VERSION_STRING.as_str(),
            node_uptime,
            node_state,
        }
    }
}

/// Minimal info of a `Block`.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MinimalBlockInfo {
    hash: BlockHash,
    timestamp: Timestamp,
    era_id: EraId,
    height: u64,
    state_root_hash: Digest,
    creator: PublicKey,
}

impl From<Block> for MinimalBlockInfo {
    fn from(block: Block) -> Self {
        MinimalBlockInfo {
            hash: *block.hash(),
            timestamp: block.header().timestamp(),
            era_id: block.header().era_id(),
            height: block.header().height(),
            state_root_hash: *block.header().state_root_hash(),
            creator: block.body().proposer().clone(),
        }
    }
}

/// Result for "info_get_status" RPC response.
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetStatusResult {
    /// The RPC API version.
    #[schemars(with = "String")]
    pub api_version: ProtocolVersion,
    /// The chainspec name.
    pub chainspec_name: String,
    /// The state root hash used at the start of the current session.
    #[deprecated(since = "1.5.0")]
    pub starting_state_root_hash: Digest,
    /// The node ID and network address of each connected peer.
    pub peers: PeersMap,
    /// The minimal info of the last block from the linear chain.
    pub last_added_block_info: Option<MinimalBlockInfo>,
    /// Our public signing key.
    pub our_public_signing_key: Option<PublicKey>,
    /// The next round length if this node is a validator.
    pub round_length: Option<TimeDiff>,
    /// Information about the next scheduled upgrade.
    pub next_upgrade: Option<NextUpgrade>,
    /// The compiled node version.
    pub build_version: String,
    /// Time that passed since the node has started.
    pub uptime: TimeDiff,
    /// The current state of node.
    pub node_state: NodeState,
}

impl GetStatusResult {
    #[allow(deprecated)]
    pub(crate) fn new(status_feed: StatusFeed, api_version: ProtocolVersion) -> Self {
        GetStatusResult {
            api_version,
            chainspec_name: status_feed.chainspec_info.name,
            starting_state_root_hash: Digest::from([0u8; 32]),
            peers: PeersMap::from(status_feed.peers),
            last_added_block_info: status_feed.last_added_block.map(Into::into),
            our_public_signing_key: status_feed.our_public_signing_key,
            round_length: status_feed.round_length,
            next_upgrade: status_feed.chainspec_info.next_upgrade,
            uptime: status_feed.node_uptime.into(),
            node_state: status_feed.node_state,
            #[cfg(not(test))]
            build_version: crate::VERSION_STRING.clone(),

            //  Prevent these values from changing between test sessions
            #[cfg(test)]
            build_version: String::from("1.0.0-xxxxxxxxx@DEBUG"),
        }
    }
}

impl DocExample for GetStatusResult {
    fn doc_example() -> &'static Self {
        &*GET_STATUS_RESULT
    }
}
