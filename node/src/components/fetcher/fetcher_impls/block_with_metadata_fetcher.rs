use std::{collections::HashMap, time::Duration};

use crate::{
    components::fetcher::{
        metrics::Metrics, Event, FetchResponder, Fetcher, ItemFetcher, ItemHandle,
    },
    effect::{requests::StorageRequest, EffectBuilder, EffectExt, Effects},
    types::{BlockWithMetadata, NodeId},
};

impl ItemFetcher<BlockWithMetadata> for Fetcher<BlockWithMetadata> {
    const SAFE_TO_RESPOND_TO_ALL: bool = false;

    fn item_handles(
        &mut self,
    ) -> &mut HashMap<u64, HashMap<NodeId, ItemHandle<BlockWithMetadata>>> {
        &mut self.item_handles
    }

    fn metrics(&mut self) -> &Metrics {
        &self.metrics
    }

    fn peer_timeout(&self) -> Duration {
        self.get_from_peer_timeout
    }

    fn get_from_storage<REv>(
        &mut self,
        effect_builder: EffectBuilder<REv>,
        id: u64,
        peer: NodeId,
        _validation_metadata: (),
        responder: FetchResponder<BlockWithMetadata>,
    ) -> Effects<Event<BlockWithMetadata>>
    where
        REv: From<StorageRequest> + Send,
    {
        todo!()
        // let fault_tolerance_fraction = self.fault_tolerance_fraction;
        // async move {
        //     let block_with_metadata = effect_builder
        //         .get_block_with_metadata_from_storage_by_height(id, false)
        //         .await?;
        //     has_enough_block_signatures(
        //         effect_builder,
        //         block_with_metadata.block.header(),
        //         &block_with_metadata.block_signatures,
        //         fault_tolerance_fraction,
        //     )
        //     .await
        //     .then_some(block_with_metadata)
        // }
        // .event(move |result| Event::GetFromStorageResult {
        //     id,
        //     peer,
        //     maybe_item: Box::new(result),
        //     responder,
        // })
    }
}