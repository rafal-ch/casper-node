//! Networking debug insights.
//!
//! The `insights` module exposes some internals of the networking component, mainly for inspection
//! through the diagnostics console. It should specifically not be used for any business logic and
//! affordances made in other corners of the `small_network` module to allow collecting these
//! insights should neither be abused just because they are available.

use std::{
    collections::{BTreeSet, HashSet},
    fmt::{self, Debug, Display, Formatter},
    net::SocketAddr,
    time::SystemTime,
};

use casper_types::{EraId, PublicKey};
use serde::Serialize;

use crate::{
    types::NodeId,
    utils::{opt_display::OptDisplay, DisplayIter, TimeAnchor},
};

use super::{
    error::ConnectionError, message::ConsensusKeyPair, outgoing::OutgoingState,
    symmetry::ConnectionSymmetry, OutgoingHandle, Payload, SmallNetwork,
};

/// A collection of insights into the active networking component.
#[derive(Debug, Serialize)]
pub(crate) struct NetworkInsights {
    /// The nodes current ID.
    our_id: NodeId,
    /// Whether or not a network CA was present (is a private network).
    network_ca: bool,
    /// The public address of the node.
    public_addr: SocketAddr,
    /// The fingerprint of a consensus key installed.
    consensus_pub_key: Option<PublicKey>,
    /// The active era as seen by the networking component.
    net_active_era: EraId,
    /// The list of node IDs that are being preferred due to being active validators.
    priviledged_active_outgoing_nodes: Option<HashSet<PublicKey>>,
    /// The list of node IDs that are being preferred due to being upcoming validators.
    priviledged_upcoming_outgoing_nodes: Option<HashSet<PublicKey>>,
    /// The amount of bandwidth allowance currently buffered, ready to be spent.
    unspent_bandwidth_allowance_bytes: Option<i64>,
    /// Map of outgoing connections, along with their current state.
    outgoing_connections: Vec<(SocketAddr, OutgoingInsight)>,
    /// Map of incoming connections.
    connection_symmetries: Vec<(NodeId, ConnectionSymmetryInsight)>,
}

/// Insight into an outgoing connection.
#[derive(Debug, Serialize)]
struct OutgoingInsight {
    /// Whether or not the address is marked unforgettable.
    unforgettable: bool,
    /// The current connection state.
    state: OutgoingStateInsight,
}

/// The state of an outgoing connection, reduced to exportable insights.
#[derive(Debug, Serialize)]
enum OutgoingStateInsight {
    Connecting {
        failures_so_far: u8,
        since: SystemTime,
    },
    Waiting {
        failures_so_far: u8,
        error: Option<String>,
        last_failure: SystemTime,
    },
    Connected {
        peer_id: NodeId,
        peer_addr: SocketAddr,
    },
    Blocked {
        since: SystemTime,
        justification: String,
    },
    Loopback,
}

fn time_delta(now: SystemTime, then: SystemTime) -> impl Display {
    OptDisplay::new(
        now.duration_since(then)
            .map(humantime::format_duration)
            .ok(),
        "err",
    )
}

impl OutgoingStateInsight {
    /// Constructs a new outgoing state insight from a given outgoing state.
    fn from_outgoing_state(
        anchor: &TimeAnchor,
        state: &OutgoingState<OutgoingHandle, ConnectionError>,
    ) -> Self {
        match state {
            OutgoingState::Connecting {
                failures_so_far,
                since,
            } => OutgoingStateInsight::Connecting {
                failures_so_far: *failures_so_far,
                since: anchor.convert(*since),
            },
            OutgoingState::Waiting {
                failures_so_far,
                error,
                last_failure,
            } => OutgoingStateInsight::Waiting {
                failures_so_far: *failures_so_far,
                error: error.as_ref().map(ToString::to_string),
                last_failure: anchor.convert(*last_failure),
            },
            OutgoingState::Connected { peer_id, handle } => OutgoingStateInsight::Connected {
                peer_id: *peer_id,
                peer_addr: handle.peer_addr,
            },
            OutgoingState::Blocked {
                since,
                justification,
            } => OutgoingStateInsight::Blocked {
                since: anchor.convert(*since),
                justification: justification.to_string(),
            },
            OutgoingState::Loopback => OutgoingStateInsight::Loopback,
        }
    }

    /// Formats the outgoing state insight with times relative to a given timestamp.
    fn fmt_time_relative(&self, now: SystemTime, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OutgoingStateInsight::Connecting {
                failures_so_far,
                since,
            } => write!(
                f,
                "connecting (fails: {}), since {}",
                failures_so_far,
                time_delta(now, *since)
            ),
            OutgoingStateInsight::Waiting {
                failures_so_far,
                error,
                last_failure,
            } => write!(
                f,
                "waiting (fails: {}, last error: {}), since {}",
                failures_so_far,
                OptDisplay::new(error.as_ref(), "none"),
                time_delta(now, *last_failure)
            ),
            OutgoingStateInsight::Connected { peer_id, peer_addr } => {
                write!(f, "connected -> {} @ {}", peer_id, peer_addr)
            }
            OutgoingStateInsight::Blocked {
                since,
                justification,
            } => {
                write!(
                    f,
                    "blocked since {}: {}",
                    time_delta(now, *since),
                    justification
                )
            }
            OutgoingStateInsight::Loopback => f.write_str("loopback"),
        }
    }
}

/// Describes whether a connection is uni- or bi-directional.
#[derive(Debug, Serialize)]
pub(super) enum ConnectionSymmetryInsight {
    IncomingOnly {
        since: SystemTime,
        peer_addrs: BTreeSet<SocketAddr>,
    },
    OutgoingOnly {
        since: SystemTime,
    },
    Symmetric {
        peer_addrs: BTreeSet<SocketAddr>,
    },
    Gone,
}

impl ConnectionSymmetryInsight {
    /// Creates a new insight from a given connection symmetry.
    fn from_connection_symmetry(anchor: &TimeAnchor, sym: &ConnectionSymmetry) -> Self {
        match sym {
            ConnectionSymmetry::IncomingOnly { since, peer_addrs } => {
                ConnectionSymmetryInsight::IncomingOnly {
                    since: anchor.convert(*since),
                    peer_addrs: peer_addrs.clone(),
                }
            }
            ConnectionSymmetry::OutgoingOnly { since } => ConnectionSymmetryInsight::OutgoingOnly {
                since: anchor.convert(*since),
            },
            ConnectionSymmetry::Symmetric { peer_addrs } => ConnectionSymmetryInsight::Symmetric {
                peer_addrs: peer_addrs.clone(),
            },
            ConnectionSymmetry::Gone => ConnectionSymmetryInsight::Gone,
        }
    }

    /// Formats the connection symmetry insight with times relative to a given timestamp.
    fn fmt_time_relative(&self, now: SystemTime, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionSymmetryInsight::IncomingOnly { since, peer_addrs } => write!(
                f,
                "<- {} (since {})",
                DisplayIter::new(peer_addrs.iter()),
                time_delta(now, *since)
            ),
            ConnectionSymmetryInsight::OutgoingOnly { since } => {
                write!(f, "-> (since {})", time_delta(now, *since))
            }
            ConnectionSymmetryInsight::Symmetric { peer_addrs } => {
                write!(f, "<> {}", DisplayIter::new(peer_addrs.iter()))
            }
            ConnectionSymmetryInsight::Gone => f.write_str("gone"),
        }
    }
}

impl NetworkInsights {
    /// Collect networking insights from a given networking component.
    pub(super) fn collect_from_component<REv, P>(net: &SmallNetwork<REv, P>) -> Self
    where
        P: Payload,
    {
        // Since we are at the top level of the component, we gain access to inner values of the
        // respective structs. We abuse this to gain debugging insights. Note: If limiters are no
        // longer a `trait`, the trait methods can be removed as well in favor of direct access.
        let (priviledged_active_outgoing_nodes, priviledged_upcoming_outgoing_nodes) = net
            .outgoing_limiter
            .debug_inspect_validators()
            .map(|(a, b)| (Some(a), Some(b)))
            .unwrap_or_default();

        let anchor = TimeAnchor::now();

        let outgoing_connections = net
            .outgoing_manager
            .outgoing
            .iter()
            .map(|(addr, outgoing)| {
                let state = OutgoingStateInsight::from_outgoing_state(&anchor, &outgoing.state);
                (
                    *addr,
                    OutgoingInsight {
                        unforgettable: outgoing.is_unforgettable,
                        state,
                    },
                )
            })
            .collect();

        let connection_symmetries = net
            .connection_symmetries
            .iter()
            .map(|(id, sym)| {
                (
                    *id,
                    ConnectionSymmetryInsight::from_connection_symmetry(&anchor, sym),
                )
            })
            .collect();

        NetworkInsights {
            our_id: net.context.our_id,
            network_ca: net.context.network_ca.is_some(),
            public_addr: net.context.public_addr,
            consensus_pub_key: net
                .context
                .consensus_keys
                .as_ref()
                .map(ConsensusKeyPair::public_key)
                .cloned(),
            net_active_era: net.active_era,
            priviledged_active_outgoing_nodes,
            priviledged_upcoming_outgoing_nodes,
            unspent_bandwidth_allowance_bytes: net
                .outgoing_limiter
                .debug_inspect_unspent_allowance(),
            outgoing_connections,
            connection_symmetries,
        }
    }
}

impl Display for NetworkInsights {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let now = SystemTime::now();

        if !self.network_ca {
            f.write_str("Public ")?;
        } else {
            f.write_str("Private ")?;
        }
        writeln!(f, "node {} @ {}", self.our_id, self.public_addr)?;
        writeln!(
            f,
            "active era: {}  consensus key: {} unspent_bandwidth_allowance_bytes: {}",
            self.net_active_era,
            DisplayIter::new(self.consensus_pub_key.as_ref()),
            OptDisplay::new(self.unspent_bandwidth_allowance_bytes, "inactive"),
        )?;
        let active = self
            .priviledged_active_outgoing_nodes
            .as_ref()
            .map(HashSet::iter)
            .map(DisplayIter::new);
        writeln!(
            f,
            "priviledged active: {}",
            OptDisplay::new(active, "inactive")
        )?;
        let upcoming = self
            .priviledged_upcoming_outgoing_nodes
            .as_ref()
            .map(HashSet::iter)
            .map(DisplayIter::new);
        writeln!(
            f,
            "priviledged upcoming: {}",
            OptDisplay::new(upcoming, "inactive")
        )?;

        f.write_str("outgoing connections:\n")?;
        writeln!(f, "address                  uf     state")?;
        for (addr, outgoing) in &self.outgoing_connections {
            write!(f, "{:23}  {:5}  ", addr, outgoing.unforgettable,)?;
            outgoing.state.fmt_time_relative(now, f)?;
            f.write_str("\n")?;
        }

        f.write_str("connection symmetries:\n")?;
        writeln!(f, "peer ID         symmetry")?;
        for (peer_id, symmetry) in &self.connection_symmetries {
            write!(f, "{:10}  ", peer_id)?;
            symmetry.fmt_time_relative(now, f)?;
            f.write_str("\n")?;
        }

        Ok(())
    }
}