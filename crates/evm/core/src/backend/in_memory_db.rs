//! In-memory database.

use crate::state_snapshot::StateSnapshots;
use alloy_primitives::{ B256, U256};
use foundry_fork_db::DatabaseError;
use revm::{
    bytecode::Bytecode,
    database::{CacheDB, DatabaseRef, EmptyDB},
    database_interface::{MultiChainDatabase, MultiChainDatabaseCommit, MultiChainDatabaseRef},
    primitives::{ChainAddress, HashMap as Map},
    state::{Account, AccountInfo},
};

/// Type alias for an in-memory database.
///
/// See [`EmptyDBWrapper`].
pub type FoundryEvmInMemoryDB = CacheDB<EmptyDBWrapper>;

/// In-memory [`Database`] for Anvil.
///
/// This acts like a wrapper type for [`FoundryEvmInMemoryDB`] but is capable of applying snapshots.
#[derive(Debug)]
pub struct MemDb {
    pub inner: FoundryEvmInMemoryDB,
    pub state_snapshots: StateSnapshots<FoundryEvmInMemoryDB>,
}

impl Default for MemDb {
    fn default() -> Self {
        Self { inner: CacheDB::new(Default::default()), state_snapshots: Default::default() }
    }
}

// XXX FIXME YSG 
impl MultiChainDatabaseRef for MemDb {
    type Error = DatabaseError;

    fn basic_ref_multi(&self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        MultiChainDatabaseRef::basic_ref_multi(&self.inner, address)
    }

    fn code_by_hash_ref_multi(
        &self,
        chain_id: u64,
        code_hash: B256,
    ) -> Result<Bytecode, Self::Error> {
        MultiChainDatabaseRef::code_by_hash_ref_multi(&self.inner, chain_id, code_hash)
    }

    fn storage_ref_multi(&self, address: ChainAddress, index: U256) -> Result<U256, Self::Error> {
        MultiChainDatabaseRef::storage_ref_multi(&self.inner, address, index)
    }

    fn block_hash_ref_multi(&self, chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        MultiChainDatabaseRef::block_hash_ref(&self.inner, chain_id, number)
    }
}

impl MultiChainDatabase for MemDb {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        // Note: this will always return `Some(AccountInfo)`, See `EmptyDBWrapper`
        MultiChainDatabase::basic_multi(&mut self.inner, address)
    }

    fn code_by_hash_multi(
        &mut self,
        chain_id: u64,
        code_hash: B256,
    ) -> Result<Bytecode, Self::Error> {
        MultiChainDatabase::code_by_hash_multi(&mut self.inner, chain_id, code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: U256) -> Result<U256, Self::Error> {
        MultiChainDatabase::storage_multi(&mut self.inner, address, index)
    }

    fn block_hash_multi(&mut self, chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        MultiChainDatabase::block_hash_multi(&mut self.inner, chain_id, number)
    }
}

impl MultiChainDatabaseCommit for MemDb {
    fn commit_multi(&mut self, changes: Map<ChainAddress, Account>) {
        MultiChainDatabaseCommit::commit(&mut self.inner, changes)
    }
}

/// An empty database that always returns default values when queried.
///
/// This is just a simple wrapper for `revm::EmptyDB` but implements `DatabaseError` instead, this
/// way we can unify all different `Database` impls
///
/// This will also _always_ return `Some(AccountInfo)`:
///
/// The [`Database`] implementation for `CacheDB` manages an `AccountState` for the
/// `DbAccount`, this will be set to `AccountState::NotExisting` if the account does not exist yet.
/// This is because there's a distinction between "non-existing" and "empty",
/// see <https://github.com/bluealloy/revm/blob/8f4348dc93022cffb3730d9db5d3ab1aad77676a/crates/revm/src/db/in_memory_db.rs#L81-L83>.
/// If an account is `NotExisting`, `Database::basic_ref` will always return `None` for the
/// requested `AccountInfo`.
///
/// To prevent this, we ensure that a missing account is never marked as `NotExisting` by always
/// returning `Some` with this type, which will then insert a default [`AccountInfo`] instead
/// of one marked as `AccountState::NotExisting`.
#[derive(Clone, Debug, Default)]
pub struct EmptyDBWrapper(EmptyDB);

impl MultiChainDatabaseRef for EmptyDBWrapper {
    type Error = DatabaseError;

    fn basic_ref_multi(&self, _address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        // Note: this will always return `Some(AccountInfo)`, for the reason explained above
        Ok(Some(AccountInfo::default()))
    }

    fn code_by_hash_ref_multi(
        &self,
        chain_id: u64,
        code_hash: B256,
    ) -> Result<Bytecode, Self::Error> {
        Ok(self.0.code_by_hash_ref_multi(chain_id, code_hash)?)
    }
    fn storage_ref_multi(&self, address: ChainAddress, index: U256) -> Result<U256, Self::Error> {
        Ok(self.0.storage_ref_multi(address, index)?)
    }

    fn block_hash_ref_multi(&self, chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        Ok(self.0.block_hash_ref_multi(chain_id, number)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::b256;

    /// Ensures the `Database(Ref)` implementation for `revm::CacheDB` works as expected
    ///
    /// Demonstrates how calling `Database::basic` works if an account does not exist
    #[test]
    fn cache_db_insert_basic_non_existing() {
        let mut db = CacheDB::new(EmptyDB::default());
        let address = ChainAddress(1, Address::random());
        // call `basic` on a non-existing account
        let info = MultiChainDatabase::basic_multi(&mut db, address).unwrap();
        assert!(info.is_none());
        let mut info = info.unwrap_or_default();
        info.balance = U256::from(500u64);

        // insert the modified account info
        db.insert_account_info(address, info);

        // when fetching again, the `AccountInfo` is still `None` because the state of the account
        // is `AccountState::NotExisting`, see <https://github.com/bluealloy/revm/blob/8f4348dc93022cffb3730d9db5d3ab1aad77676a/crates/revm/src/db/in_memory_db.rs#L217-L226>
        let info = MultiChainDatabase::basic_multi(&mut db, address).unwrap();
        assert!(info.is_none());
    }

    /// Demonstrates how to insert a new account but not mark it as non-existing
    #[test]
    fn cache_db_insert_basic_default() {
        let mut db = CacheDB::new(EmptyDB::default());
        let address = ChainAddress(1, Address::random());

        // We use `basic_multi_ref` here to ensure that the account is not marked as `NotExisting`.
        let info = MultiChainDatabaseRef::basic_multi_ref(&db, address).unwrap();
        assert!(info.is_none());
        let mut info = info.unwrap_or_default();
        info.balance = U256::from(500u64);

        // insert the modified account info
        db.insert_account_info(address, info.clone());

        let loaded = MultiChainDatabase::basic_multi(&mut db, address).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), info)
    }

    /// Demonstrates that `Database::basic` for `MemDb` will always return the `AccountInfo`
    #[test]
    fn mem_db_insert_basic_default() {
        let mut db = MemDb::default();
        let address = ChainAddress(1, Address::from_word(b256!(
            "0x000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045"
        )));

        let info = MultiChainDatabase::basic_multi(&mut db, address).unwrap();
        // We know info exists, as MemDb always returns `Some(AccountInfo)` due to the
        // `EmptyDbWrapper`.
        assert!(info.is_some());
        let mut info = info.unwrap();
        info.balance = U256::from(500u64);

        // insert the modified account info
        db.inner.insert_account_info(address, info.clone());

        let loaded = MultiChainDatabase::basic_multi(&mut db, address).unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap(), info)
    }
}
