use std::collections::BTreeMap;

use num_rational::Ratio;

use casper_types::{PublicKey, U512};

use crate::{
    components::consensus::{self, error::FinalitySignatureError},
    types::BlockSignatures,
};

/// Collects finality signatures and detects if they are sufficient, i.e. if the finality
/// signatures' total weight exceeds the threshold specified in `finality_threshold_fraction`.
pub(crate) struct BlockSignaturesCollector(Option<BlockSignatures>);

impl BlockSignaturesCollector {
    pub(crate) fn new() -> Self {
        Self(None)
    }

    // Adds new set of finality signatures. Returns `true` if any of the signatures
    // was not previously stored in self.
    pub(crate) fn add(&mut self, signatures: BlockSignatures) -> bool {
        match &mut self.0 {
            None => {
                self.0 = Some(signatures);
                true
            }
            Some(old_sigs) => {
                let mut any_inserted = false;
                for (pub_key, signature) in signatures.proofs {
                    // We don't compare signatures. The assumption is that within the same block
                    // it is not correct to have different signatures from the same validator.
                    if old_sigs.insert_proof(pub_key, signature).is_none() {
                        any_inserted = true;
                    }
                }
                any_inserted
            }
        }
    }

    pub(crate) fn check_if_sufficient(
        &self,
        validator_weights: &BTreeMap<PublicKey, U512>,
        finality_threshold_fraction: Ratio<u64>,
    ) -> bool {
        self.0.as_ref().map_or(false, |sigs| {
            are_signatures_sufficient_for_sync_to_genesis(
                consensus::check_sufficient_finality_signatures(
                    validator_weights,
                    finality_threshold_fraction,
                    sigs,
                ),
            )
        })
    }

    pub(crate) fn into_inner(self) -> Option<BlockSignatures> {
        self.0
    }
}

// Returns true if the output from consensus can be interpreted
// as sufficient finality signatures for the sync to genesis process.
fn are_signatures_sufficient_for_sync_to_genesis(
    consensus_verdict: Result<(), FinalitySignatureError>,
) -> bool {
    match consensus_verdict {
        Ok(_) | Err(FinalitySignatureError::TooManySignatures { .. }) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use casper_types::{testing::TestRng, AsymmetricType, EraId, PublicKey, Signature, U512};
    use num_rational::Ratio;

    use crate::{
        components::{
            chain_synchronizer::block_signatures_collector::{
                are_signatures_sufficient_for_sync_to_genesis, BlockSignaturesCollector,
            },
            consensus::error::FinalitySignatureError,
        },
        types::{BlockHash, BlockSignatures},
    };

    #[test]
    fn validates_signatures_sufficiency_for_sync_to_genesis() {
        let consensus_verdict = Ok(());
        assert!(are_signatures_sufficient_for_sync_to_genesis(
            consensus_verdict
        ));

        let mut rng = TestRng::new();
        let consensus_verdict = Err(FinalitySignatureError::TooManySignatures {
            trusted_validator_weights: BTreeMap::new(),
            block_signatures: Box::new(BlockSignatures::new(
                BlockHash::random(&mut rng),
                EraId::from(0),
            )),
            signature_weight: Box::new(U512::from(0u16)),
            weight_minus_minimum: Box::new(U512::from(0u16)),
            total_validator_weight: Box::new(U512::from(0u16)),
            finality_threshold_fraction: Ratio::new_raw(1, 2),
        });
        assert!(are_signatures_sufficient_for_sync_to_genesis(
            consensus_verdict
        ));

        let consensus_verdict = Err(FinalitySignatureError::InsufficientWeightForFinality {
            trusted_validator_weights: BTreeMap::new(),
            block_signatures: Box::new(BlockSignatures::new(
                BlockHash::random(&mut rng),
                EraId::from(0),
            )),
            signature_weight: Box::new(U512::from(0u16)),
            total_validator_weight: Box::new(U512::from(0u16)),
            finality_threshold_fraction: Ratio::new_raw(1, 2),
        });
        assert!(!are_signatures_sufficient_for_sync_to_genesis(
            consensus_verdict
        ));

        let consensus_verdict = Err(FinalitySignatureError::BogusValidator {
            trusted_validator_weights: BTreeMap::new(),
            block_signatures: Box::new(BlockSignatures::new(
                BlockHash::random(&mut rng),
                EraId::from(0),
            )),
            bogus_validator_public_key: Box::new(PublicKey::random_ed25519(&mut rng)),
        });
        assert!(!are_signatures_sufficient_for_sync_to_genesis(
            consensus_verdict
        ));
    }

    #[test]
    fn detects_insertion_of_new_signatures() {
        let mut rng = TestRng::new();
        let mut block_signatures_collector = BlockSignaturesCollector::new();

        let block_hash_1 = BlockHash::random(&mut rng);

        let public_key_1 = PublicKey::random_ed25519(&mut rng);
        let public_key_2 = PublicKey::random_ed25519(&mut rng);
        let signature_body = Signature::system();

        // Insert first set of signatures with just one proof. Expect true.
        let mut signatures = BlockSignatures::new(block_hash_1, EraId::from(0));
        signatures.insert_proof(public_key_1, signature_body);
        assert!(block_signatures_collector.add(signatures.clone()));

        // Insert the same set of signatures with proofs and expect false.
        assert!(!block_signatures_collector.add(signatures.clone()));

        // Add proof from another validator and expect true.
        signatures.insert_proof(public_key_2.clone(), signature_body);
        assert!(block_signatures_collector.add(signatures.clone()));

        // Trying again with the same set, so expect false.
        assert!(!block_signatures_collector.add(signatures.clone()));

        // Add another proof for the same public key. This shouldn't happen in the system, but we
        // test to document how the system behaves.
        signatures.insert_proof(public_key_2, signature_body);
        assert!(!block_signatures_collector.add(signatures.clone()));
    }
}
