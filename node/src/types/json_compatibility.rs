//! Types which are serializable to JSON, which map to types defined outside this module.

mod account;
mod auction_state;
mod contracts;
mod stored_value;

use casper_types::{contracts::NamedKeys, NamedKeyV1};

pub use account::Account;
pub use auction_state::AuctionState;
pub use contracts::{Contract, ContractPackage};
pub use stored_value::StoredValue;

/// A helper function to change NamedKeys into a Vec<NamedKey>
pub fn vectorize(keys: &NamedKeys) -> Vec<NamedKeyV1> {
    let named_keys = keys
        .iter()
        .map(|(name, key)| NamedKeyV1 {
            name: name.clone(),
            key: key.to_formatted_string(),
        })
        .collect();
    named_keys
}
