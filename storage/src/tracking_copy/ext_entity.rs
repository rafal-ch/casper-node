use std::{collections::BTreeSet, convert::TryFrom};
use tracing::error;

use casper_types::{
    account::AccountHash,
    addressable_entity::{
        ActionThresholds, AssociatedKeys, MessageTopics, NamedKeyAddr, NamedKeyValue, NamedKeys,
        Weight,
    },
    bytesrepr,
    system::{handle_payment::ACCUMULATION_PURSE_KEY, AUCTION, HANDLE_PAYMENT, MINT},
    AccessRights, Account, AddressableEntity, AddressableEntityHash, ByteCodeHash, CLValue,
    ContextAccessRights, EntityAddr, EntityKind, EntityVersions, EntryPoints, Groups, Key, Package,
    PackageHash, PackageStatus, Phase, ProtocolVersion, PublicKey, StoredValue,
    StoredValueTypeMismatch, URef,
};

use crate::{
    global_state::{error::Error as GlobalStateError, state::StateReader},
    tracking_copy::{TrackingCopy, TrackingCopyError, TrackingCopyExt},
    AddressGenerator,
};

/// Fees purse handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeesPurseHandling {
    ToProposer(AccountHash),
    Accumulate,
    Burn,
    None(URef),
}

/// Higher-level operations on the state via a `TrackingCopy`.
pub trait TrackingCopyEntityExt<R> {
    /// The type for the returned errors.
    type Error;

    /// Gets an addressable entity by address.
    fn get_addressable_entity(
        &mut self,
        entity_addr: EntityAddr,
    ) -> Result<AddressableEntity, Self::Error>;

    /// Gets an addressable entity by hash.
    fn get_addressable_entity_by_hash(
        &mut self,
        addressable_entity_hash: AddressableEntityHash,
    ) -> Result<AddressableEntity, Self::Error>;

    /// Gets the entity hash for an account hash.
    fn get_entity_hash_by_account_hash(
        &mut self,
        account_hash: AccountHash,
    ) -> Result<AddressableEntityHash, Self::Error>;

    /// Gets the entity for a given account by its account hash.
    fn get_addressable_entity_by_account_hash(
        &mut self,
        protocol_version: ProtocolVersion,
        account_hash: AccountHash,
    ) -> Result<AddressableEntity, Self::Error>;

    /// Get entity if authorized, else error.
    fn get_authorized_addressable_entity(
        &mut self,
        protocol_version: ProtocolVersion,
        account_hash: AccountHash,
        authorization_keys: &BTreeSet<AccountHash>,
        administrative_accounts: &BTreeSet<AccountHash>,
    ) -> Result<(AddressableEntity, AddressableEntityHash), Self::Error>;

    /// Migrate the NamedKeys for a Contract or Account.
    fn migrate_named_keys(
        &mut self,
        entity_addr: EntityAddr,
        named_keys: NamedKeys,
    ) -> Result<(), Self::Error>;

    fn migrate_account(
        &mut self,
        account_hash: AccountHash,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error>;
    fn create_new_addressable_entity_on_transfer(
        &mut self,
        account_hash: AccountHash,
        main_purse: URef,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error>;

    fn create_addressable_entity_from_account(
        &mut self,
        account: Account,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error>;

    /// Returns entity, named keys, and access rights for the system.
    fn system_entity(
        &mut self,
        protocol_version: ProtocolVersion,
    ) -> Result<(AddressableEntity, NamedKeys, ContextAccessRights), TrackingCopyError>;

    /// Returns entity, named keys, and access rights.
    fn resolved_entity(
        &mut self,
        protocol_version: ProtocolVersion,
        initiating_address: AccountHash,
        authorization_keys: &BTreeSet<AccountHash>,
        administrative_accounts: &BTreeSet<AccountHash>,
    ) -> Result<(AddressableEntity, NamedKeys, ContextAccessRights), TrackingCopyError>;

    /// Returns fee purse.
    fn fees_purse(
        &mut self,
        protocol_version: ProtocolVersion,
        fees_purse_handling: FeesPurseHandling,
    ) -> Result<URef, TrackingCopyError>;
}

impl<R> TrackingCopyEntityExt<R> for TrackingCopy<R>
where
    R: StateReader<Key, StoredValue, Error = GlobalStateError>,
{
    type Error = TrackingCopyError;

    fn get_addressable_entity(
        &mut self,
        entity_addr: EntityAddr,
    ) -> Result<AddressableEntity, Self::Error> {
        let key = Key::AddressableEntity(entity_addr);

        match self.read(&key)? {
            Some(StoredValue::AddressableEntity(entity)) => Ok(entity),
            Some(other) => Err(TrackingCopyError::TypeMismatch(
                StoredValueTypeMismatch::new(
                    "AddressableEntity or Contract".to_string(),
                    other.type_name(),
                ),
            )),
            None => Err(TrackingCopyError::KeyNotFound(key)),
        }
    }

    fn get_addressable_entity_by_hash(
        &mut self,
        entity_hash: AddressableEntityHash,
    ) -> Result<AddressableEntity, Self::Error> {
        let entity_addr = if self
            .get_system_entity_registry()?
            .has_contract_hash(&entity_hash)
        {
            EntityAddr::new_system(entity_hash.value())
        } else {
            EntityAddr::new_smart_contract(entity_hash.value())
        };

        self.get_addressable_entity(entity_addr)
    }

    fn get_entity_hash_by_account_hash(
        &mut self,
        account_hash: AccountHash,
    ) -> Result<AddressableEntityHash, Self::Error> {
        let account_key = Key::Account(account_hash);
        match self.get(&account_key)? {
            Some(StoredValue::CLValue(cl_value)) => {
                let entity_key = CLValue::into_t::<Key>(cl_value)?;
                let entity_hash = AddressableEntityHash::try_from(entity_key)
                    .map_err(|_| TrackingCopyError::BytesRepr(bytesrepr::Error::Formatting))?;

                Ok(entity_hash)
            }
            Some(other) => Err(TrackingCopyError::TypeMismatch(
                StoredValueTypeMismatch::new("CLValue".to_string(), other.type_name()),
            )),
            None => Err(TrackingCopyError::KeyNotFound(account_key)),
        }
    }

    fn get_addressable_entity_by_account_hash(
        &mut self,
        protocol_version: ProtocolVersion,
        account_hash: AccountHash,
    ) -> Result<AddressableEntity, Self::Error> {
        let account_key = Key::Account(account_hash);

        let contract_key = match self.get(&account_key)? {
            Some(StoredValue::CLValue(contract_key_as_cl_value)) => {
                CLValue::into_t::<Key>(contract_key_as_cl_value)?
            }
            Some(StoredValue::Account(account)) => {
                // do a legacy account migration
                let mut generator =
                    AddressGenerator::new(account.main_purse().addr().as_ref(), Phase::System);

                let byte_code_hash = ByteCodeHash::default();
                let entity_hash = AddressableEntityHash::new(generator.new_hash_address());
                let package_hash = PackageHash::new(generator.new_hash_address());

                let entry_points = EntryPoints::new();

                self.migrate_named_keys(
                    EntityAddr::Account(entity_hash.value()),
                    account.named_keys().clone(),
                )?;

                let entity = AddressableEntity::new(
                    package_hash,
                    byte_code_hash,
                    entry_points,
                    protocol_version,
                    account.main_purse(),
                    account.associated_keys().clone().into(),
                    account.action_thresholds().clone().into(),
                    MessageTopics::default(),
                    EntityKind::Account(account_hash),
                );

                let access_key = generator.new_uref(AccessRights::READ_ADD_WRITE);

                let package = {
                    let mut package = Package::new(
                        access_key,
                        EntityVersions::default(),
                        BTreeSet::default(),
                        Groups::default(),
                        PackageStatus::Locked,
                    );
                    package.insert_entity_version(protocol_version.value().major, entity_hash);
                    package
                };

                let entity_key = entity.entity_key(entity_hash);

                self.write(entity_key, StoredValue::AddressableEntity(entity.clone()));
                self.write(package_hash.into(), package.into());

                let contract_by_account = match CLValue::from_t(entity_key) {
                    Ok(cl_value) => cl_value,
                    Err(error) => return Err(TrackingCopyError::CLValue(error)),
                };

                self.write(account_key, StoredValue::CLValue(contract_by_account));

                return Ok(entity);
            }

            Some(other) => {
                return Err(TrackingCopyError::TypeMismatch(
                    StoredValueTypeMismatch::new("Key".to_string(), other.type_name()),
                ));
            }
            None => return Err(TrackingCopyError::KeyNotFound(account_key)),
        };

        match self.get(&contract_key)? {
            Some(StoredValue::AddressableEntity(contract)) => Ok(contract),
            Some(other) => Err(TrackingCopyError::TypeMismatch(
                StoredValueTypeMismatch::new("Contract".to_string(), other.type_name()),
            )),
            None => Err(TrackingCopyError::KeyNotFound(contract_key)),
        }
    }

    fn get_authorized_addressable_entity(
        &mut self,
        protocol_version: ProtocolVersion,
        account_hash: AccountHash,
        authorization_keys: &BTreeSet<AccountHash>,
        administrative_accounts: &BTreeSet<AccountHash>,
    ) -> Result<(AddressableEntity, AddressableEntityHash), Self::Error> {
        let entity_record =
            self.get_addressable_entity_by_account_hash(protocol_version, account_hash)?;

        let entity_hash = self.get_entity_hash_by_account_hash(account_hash)?;

        if !administrative_accounts.is_empty()
            && administrative_accounts
                .intersection(authorization_keys)
                .next()
                .is_some()
        {
            // Exit early if there's at least a single signature coming from an admin.
            return Ok((entity_record, entity_hash));
        }

        // Authorize using provided authorization keys
        if !entity_record.can_authorize(authorization_keys) {
            return Err(Self::Error::Authorization);
        }

        // Check total key weight against deploy threshold
        if !entity_record.can_deploy_with(authorization_keys) {
            return Err(Self::Error::DeploymentAuthorizationFailure);
        }

        Ok((entity_record, entity_hash))
    }

    fn migrate_named_keys(
        &mut self,
        entity_addr: EntityAddr,
        named_keys: NamedKeys,
    ) -> Result<(), Self::Error> {
        for (string, key) in named_keys.into_inner().into_iter() {
            let entry_addr = NamedKeyAddr::new_from_string(entity_addr, string.clone())?;

            let named_key_value =
                StoredValue::NamedKey(NamedKeyValue::from_concrete_values(key, string.clone())?);

            let entry_key = Key::NamedKey(entry_addr);

            self.write(entry_key, named_key_value)
        }

        Ok(())
    }

    fn migrate_account(
        &mut self,
        account_hash: AccountHash,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error> {
        let key = Key::Account(account_hash);
        let maybe_stored_value = self.read(&key)?;

        match maybe_stored_value {
            Some(StoredValue::Account(account)) => {
                self.create_addressable_entity_from_account(account, protocol_version)
            }
            Some(StoredValue::CLValue(_)) => Ok(()),
            // This means the Account does not exist, which we consider to be
            // an authorization error. As used by the node, this type of deploy
            // will have already been filtered out, but for other EE use cases
            // and testing it is reachable.
            Some(_) => Err(Self::Error::UnexpectedStoredValueVariant),
            None => Err(Self::Error::AccountNotFound(key)),
        }
    }

    fn create_new_addressable_entity_on_transfer(
        &mut self,
        account_hash: AccountHash,
        main_purse: URef,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error> {
        let mut generator = AddressGenerator::new(main_purse.addr().as_ref(), Phase::System);

        let byte_code_hash = ByteCodeHash::default();
        let entity_hash = AddressableEntityHash::new(generator.new_hash_address());
        let package_hash = PackageHash::new(generator.new_hash_address());

        let entry_points = EntryPoints::new();

        let associated_keys = AssociatedKeys::new(account_hash, Weight::new(1));

        let action_thresholds: ActionThresholds = Default::default();

        let entity = AddressableEntity::new(
            package_hash,
            byte_code_hash,
            entry_points,
            protocol_version,
            main_purse,
            associated_keys,
            action_thresholds,
            MessageTopics::default(),
            EntityKind::Account(account_hash),
        );

        let access_key = generator.new_uref(AccessRights::READ_ADD_WRITE);

        let package = {
            let mut package = Package::new(
                access_key,
                EntityVersions::default(),
                BTreeSet::default(),
                Groups::default(),
                PackageStatus::Locked,
            );
            package.insert_entity_version(protocol_version.value().major, entity_hash);
            package
        };

        let entity_addr = EntityAddr::new_account(entity_hash.value());
        let entity_key = Key::AddressableEntity(entity_addr);

        self.write(entity_key, entity.into());
        self.write(package_hash.into(), package.into());
        let contract_by_account = match CLValue::from_t(entity_key) {
            Ok(cl_value) => cl_value,
            Err(err) => return Err(Self::Error::CLValue(err)),
        };

        self.write(
            Key::Account(account_hash),
            StoredValue::CLValue(contract_by_account),
        );
        Ok(())
    }

    fn create_addressable_entity_from_account(
        &mut self,
        account: Account,
        protocol_version: ProtocolVersion,
    ) -> Result<(), Self::Error> {
        let account_hash = account.account_hash();

        // carry forward the account hash to allow reverse lookup
        let entity_hash = AddressableEntityHash::new(account_hash.value());
        let entity_addr = EntityAddr::new_account(entity_hash.value());

        // migrate named keys -- if this fails there is no reason to proceed further.
        let named_keys = account.named_keys().clone();
        self.migrate_named_keys(entity_addr, named_keys)?;

        // write package first
        let package_hash = {
            let mut generator =
                AddressGenerator::new(account.main_purse().addr().as_ref(), Phase::System);

            let package_hash = PackageHash::new(generator.new_hash_address());
            let access_key = generator.new_uref(AccessRights::READ_ADD_WRITE);

            let mut package = Package::new(
                access_key,
                EntityVersions::default(),
                BTreeSet::default(),
                Groups::default(),
                PackageStatus::Locked,
            );
            package.insert_entity_version(protocol_version.value().major, entity_hash);
            self.write(package_hash.into(), package.into());
            package_hash
        };

        // write entity after package
        {
            // currently, addressable entities of account kind are not permitted to have bytecode
            // however, we intend to revisit this and potentially allow it in a future release
            // as a replacement for stored session.
            let byte_code_hash = ByteCodeHash::default();
            let entry_points = EntryPoints::new();

            let action_thresholds = {
                let account_threshold = account.action_thresholds().clone();
                ActionThresholds::new(
                    Weight::new(account_threshold.deployment.value()),
                    Weight::new(1u8),
                    Weight::new(account_threshold.key_management.value()),
                )
                .map_err(Self::Error::SetThresholdFailure)?
            };

            let associated_keys = AssociatedKeys::from(account.associated_keys().clone());

            let entity = AddressableEntity::new(
                package_hash,
                byte_code_hash,
                entry_points,
                protocol_version,
                account.main_purse(),
                associated_keys,
                action_thresholds,
                MessageTopics::default(),
                EntityKind::Account(account_hash),
            );
            let entity_key = entity.entity_key(entity_hash);
            let contract_by_account = match CLValue::from_t(entity_key) {
                Ok(cl_value) => cl_value,
                Err(err) => return Err(Self::Error::CLValue(err)),
            };

            self.write(entity_key, entity.into());
            self.write(
                Key::Account(account_hash),
                StoredValue::CLValue(contract_by_account),
            );
        }

        Ok(())
    }

    fn system_entity(
        &mut self,
        protocol_version: ProtocolVersion,
    ) -> Result<(AddressableEntity, NamedKeys, ContextAccessRights), TrackingCopyError> {
        let system_account_hash = PublicKey::System.to_account_hash();
        let system_entity =
            self.get_addressable_entity_by_account_hash(protocol_version, system_account_hash)?;

        let system_entity_registry = self.get_system_entity_registry()?;

        let (auction_named_keys, mut auction_access_rights) = {
            let auction_hash = match system_entity_registry.get(AUCTION).copied() {
                Some(auction_hash) => auction_hash,
                None => {
                    error!("unexpected failure; auction not found");
                    return Err(TrackingCopyError::MissingSystemContractHash(
                        AUCTION.to_string(),
                    ));
                }
            };
            let auction = self.get_addressable_entity_by_hash(auction_hash)?;
            let auction_addr = auction.entity_addr(auction_hash);
            let auction_named_keys = self.get_named_keys(auction_addr)?;
            let auction_access_rights =
                auction.extract_access_rights(auction_hash, &auction_named_keys);
            (auction_named_keys, auction_access_rights)
        };
        let (mint_named_keys, mint_access_rights) = {
            let mint_hash = match system_entity_registry.get(MINT).copied() {
                Some(mint_hash) => mint_hash,
                None => {
                    error!("unexpected failure; mint not found");
                    return Err(TrackingCopyError::MissingSystemContractHash(
                        MINT.to_string(),
                    ));
                }
            };
            let mint = self.get_addressable_entity_by_hash(mint_hash)?;
            let mint_addr = mint.entity_addr(mint_hash);
            let mint_named_keys = self.get_named_keys(mint_addr)?;
            let mint_access_rights = mint.extract_access_rights(mint_hash, &mint_named_keys);
            (mint_named_keys, mint_access_rights)
        };

        let (payment_named_keys, payment_access_rights) = {
            let payment_hash = match system_entity_registry.get(HANDLE_PAYMENT).copied() {
                Some(payment_hash) => payment_hash,
                None => {
                    error!("unexpected failure; handle payment not found");
                    return Err(TrackingCopyError::MissingSystemContractHash(
                        HANDLE_PAYMENT.to_string(),
                    ));
                }
            };
            let payment = self.get_addressable_entity_by_hash(payment_hash)?;
            let payment_addr = payment.entity_addr(payment_hash);
            let payment_named_keys = self.get_named_keys(payment_addr)?;
            let payment_access_rights =
                payment.extract_access_rights(payment_hash, &mint_named_keys);
            (payment_named_keys, payment_access_rights)
        };

        // the auction calls the mint for total supply behavior, so extending the context to include
        // mint named keys & access rights

        let mut named_keys = NamedKeys::new();
        named_keys.append(auction_named_keys);
        named_keys.append(mint_named_keys);
        named_keys.append(payment_named_keys);

        auction_access_rights.extend_access_rights(mint_access_rights.take_access_rights());
        auction_access_rights.extend_access_rights(payment_access_rights.take_access_rights());
        Ok((system_entity, named_keys, auction_access_rights))
    }

    fn resolved_entity(
        &mut self,
        protocol_version: ProtocolVersion,
        initiating_address: AccountHash,
        authorization_keys: &BTreeSet<AccountHash>,
        administrative_accounts: &BTreeSet<AccountHash>,
    ) -> Result<(AddressableEntity, NamedKeys, ContextAccessRights), TrackingCopyError> {
        if initiating_address == PublicKey::System.to_account_hash() {
            return self.system_entity(protocol_version);
        }

        let (entity, entity_hash) = self.get_authorized_addressable_entity(
            protocol_version,
            initiating_address,
            authorization_keys,
            administrative_accounts,
        )?;
        let entity_addr = entity.entity_addr(entity_hash);
        let named_keys = self.get_named_keys(entity_addr)?;
        let access_rights = entity
            .extract_access_rights(AddressableEntityHash::new(entity_addr.value()), &named_keys);
        Ok((entity, named_keys, access_rights))
    }

    fn fees_purse(
        &mut self,
        protocol_version: ProtocolVersion,
        fees_purse_handling: FeesPurseHandling,
    ) -> Result<URef, TrackingCopyError> {
        let protocol_version = protocol_version;
        let fee_handling = fees_purse_handling;
        match fee_handling {
            FeesPurseHandling::None(uref) => Ok(uref),
            FeesPurseHandling::ToProposer(proposer) => {
                let proposer_account: AddressableEntity =
                    self.get_addressable_entity_by_account_hash(protocol_version, proposer)?;

                Ok(proposer_account.main_purse())
            }
            FeesPurseHandling::Accumulate => {
                let registry = self.get_system_entity_registry()?;
                let entity_addr = {
                    let hash = match registry.get(HANDLE_PAYMENT) {
                        Some(hash) => hash,
                        None => {
                            return Err(TrackingCopyError::MissingSystemContractHash(
                                HANDLE_PAYMENT.to_string(),
                            ))
                        }
                    };
                    EntityAddr::new_system(hash.value())
                };

                let named_keys = self.get_named_keys(entity_addr)?;

                let accumulation_purse_uref = match named_keys.get(ACCUMULATION_PURSE_KEY) {
                    Some(Key::URef(accumulation_purse)) => *accumulation_purse,
                    Some(_) | None => {
                        error!(
                            "fee handling is configured to accumulate but handle payment does not \
                            have accumulation purse"
                        );
                        return Err(TrackingCopyError::NamedKeyNotFound(
                            ACCUMULATION_PURSE_KEY.to_string(),
                        ));
                    }
                };

                Ok(accumulation_purse_uref)
            }
            FeesPurseHandling::Burn => {
                // TODO: replace this with new burn logic once it merges
                Ok(casper_types::URef::default())
            }
        }
    }
}
