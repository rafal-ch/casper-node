use std::{collections::HashMap, time::Duration};

use crate::{
    components::fetcher::{
        metrics::Metrics, Event, FetchResponder, Fetcher, ItemFetcher, ItemHandle,
    },
    effect::{requests::StorageRequest, EffectBuilder, EffectExt, Effects},
    types::{Deploy, DeployHash, DeployWithFinalizedApprovals, NodeId},
};

impl ItemFetcher<Deploy> for Fetcher<Deploy> {
    const SAFE_TO_RESPOND_TO_ALL: bool = true;

    fn item_handles(&mut self) -> &mut HashMap<DeployHash, HashMap<NodeId, ItemHandle<Deploy>>> {
        &mut self.item_handles
    }

    fn metrics(&mut self) -> &Metrics {
        &self.metrics
    }

    fn peer_timeout(&self) -> Duration {
        self.get_from_peer_timeout
    }

    /// Gets a `Deploy` from the storage component.
    fn get_from_storage<REv>(
        &mut self,
        effect_builder: EffectBuilder<REv>,
        id: DeployHash,
        peer: NodeId,
        _validation_metadata: (),
        responder: FetchResponder<Deploy>,
    ) -> Effects<Event<Deploy>>
    where
        REv: From<StorageRequest> + Send,
    {
        effect_builder
            .get_deploys_from_storage(vec![id])
            .event(move |mut results| Event::GetFromStorageResult {
                id,
                peer,
                validation_metadata: (),
                maybe_item: Box::new(
                    results
                        .pop()
                        .expect("can only contain one result")
                        .map(DeployWithFinalizedApprovals::into_naive),
                ),
                responder,
            })
    }
}