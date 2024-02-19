use std::collections::BTreeSet;

use datasize::DataSize;
#[cfg(test)]
use rand::Rng;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use casper_types::testing::TestRng;
use casper_types::{
    bytesrepr::{self, FromBytes, ToBytes},
    DeployApproval,
};

/// A set of approvals that has been agreed upon by consensus to approve of a specific deploy.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, DataSize, Debug)]
pub(crate) struct FinalizedDeployApprovals(BTreeSet<DeployApproval>);

impl FinalizedDeployApprovals {
    pub(crate) fn new(approvals: BTreeSet<DeployApproval>) -> Self {
        Self(approvals)
    }

    pub(crate) fn inner(&self) -> &BTreeSet<DeployApproval> {
        &self.0
    }

    pub(crate) fn into_inner(self) -> BTreeSet<DeployApproval> {
        self.0
    }

    #[cfg(test)]
    pub(crate) fn random(rng: &mut TestRng) -> Self {
        let count = rng.gen_range(1..10);
        let approvals = (0..count)
            .into_iter()
            .map(|_| DeployApproval::random(rng))
            .collect();
        FinalizedDeployApprovals(approvals)
    }
}

impl ToBytes for FinalizedDeployApprovals {
    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.0.write_bytes(writer)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        self.0.to_bytes()
    }

    fn serialized_length(&self) -> usize {
        self.0.serialized_length()
    }
}

impl FromBytes for FinalizedDeployApprovals {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (approvals, remainder) = BTreeSet::<DeployApproval>::from_bytes(bytes)?;
        Ok((FinalizedDeployApprovals(approvals), remainder))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytesrepr_roundtrip() {
        let rng = &mut TestRng::new();
        let approvals = FinalizedDeployApprovals::random(rng);
        bytesrepr::test_serialization_roundtrip(&approvals);
    }
}