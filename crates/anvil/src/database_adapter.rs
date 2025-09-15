//! Database Adapter Module
//! 
//! This module provides a trait object based approach to avoid orphan rule violations.
//! Instead of implementing foreign traits for foreign types, we create our own
//! database trait and adapt existing types to it.

use alloy_primitives::{Address, B256, U256, map::HashMap};
use alloy_rpc_types::BlockId;
use foundry_evm::backend::{BlockchainDb, DatabaseError, DatabaseResult, RevertStateSnapshotAction, StateSnapshot};
use revm::{
    database::{CacheDB, DatabaseRef, WrapDatabaseRef}, 
    context_interface::MultiChainDatabase,
    database_interface::MultiChainDatabaseCommit,
    primitives::ChainAddress,
    state::{Account, AccountInfo},
    bytecode::Bytecode,
    Database, DatabaseCommit,
};
use foundry_evm::{backend::MemDb, fork::database::ForkedDatabase};
use std::fmt;

use crate::eth::backend::db::{
    MaybeForkedDatabase, MaybeFullDatabase, SerializableAccountRecord, SerializableBlock,
    SerializableHistoricalStates, SerializableState, SerializableTransaction, StateDb,
};

/// Our own database trait that we can implement for any type
pub trait AnvilDatabase: fmt::Debug + Send + Sync {
    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, DatabaseError>;
    fn code_by_hash_multi(&mut self, chain_id: u64, code_hash: B256) -> Result<Bytecode, DatabaseError>;
    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError>;
    fn block_hash_multi(&mut self, chain_id: u64, number: u64) -> Result<B256, DatabaseError>;
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>);
    
    // Standard database operations
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, DatabaseError>;
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, DatabaseError>;
    fn storage_ref(&self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError>;
    fn block_hash_ref(&self, number: u64) -> Result<B256, DatabaseError>;
    
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, DatabaseError>;
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, DatabaseError>;
    fn storage(&mut self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError>;
    fn block_hash(&mut self, number: u64) -> Result<B256, DatabaseError>;
    fn commit(&mut self, changes: HashMap<Address, Account>);
    
    // Anvil-specific operations
    fn insert_account(&mut self, address: Address, account: AccountInfo);
    fn set_storage_at(&mut self, address: Address, slot: B256, val: B256) -> DatabaseResult<()>;
    fn insert_block_hash(&mut self, number: U256, hash: B256);
    fn dump_state(
        &self,
        at: revm::context::BlockEnv,
        best_number: u64,
        blocks: Vec<SerializableBlock>,
        transactions: Vec<SerializableTransaction>,
        historical_states: Option<SerializableHistoricalStates>,
    ) -> DatabaseResult<Option<SerializableState>>;
    fn snapshot_state(&mut self) -> U256;
    fn revert_state(&mut self, id: U256, action: RevertStateSnapshotAction) -> bool;
    fn current_state(&self) -> StateDb;
    fn maybe_state_root(&self) -> Option<B256> { None }
}

/// Adapter for CacheDB
#[derive(Debug)]
pub struct CacheDbAdapter<T> {
    pub inner: CacheDB<T>,
}

impl<T: DatabaseRef<Error = DatabaseError> + Send + Sync + Clone + fmt::Debug> CacheDbAdapter<T> {
    pub fn new(inner: CacheDB<T>) -> Self {
        Self { inner }
    }
}

impl<T: DatabaseRef<Error = DatabaseError> + Send + Sync + Clone + fmt::Debug> AnvilDatabase for CacheDbAdapter<T> {
    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, DatabaseError> {
        use revm::Database;
        self.inner.basic(address.1)
    }
    
    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        use revm::Database;
        self.inner.code_by_hash(code_hash)
    }
    
    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        use revm::Database;
        self.inner.storage(address.1, index)
    }
    
    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, DatabaseError> {
        use revm::Database;
        self.inner.block_hash(number)
    }
    
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.inner.commit(single_chain_changes)
    }
    
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, DatabaseError> {
        self.inner.basic_ref(address)
    }
    
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        self.inner.code_by_hash_ref(code_hash)
    }
    
    fn storage_ref(&self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        self.inner.storage_ref(address, index)
    }
    
    fn block_hash_ref(&self, number: u64) -> Result<B256, DatabaseError> {
        self.inner.block_hash_ref(number)
    }
    
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, DatabaseError> {
        use revm::Database;
        self.inner.basic(address)
    }
    
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        use revm::Database;
        self.inner.code_by_hash(code_hash)
    }
    
    fn storage(&mut self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        use revm::Database;
        self.inner.storage(address, index)
    }
    
    fn block_hash(&mut self, number: u64) -> Result<B256, DatabaseError> {
        use revm::Database;
        self.inner.block_hash(number)
    }
    
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        use revm::DatabaseCommit;
        self.inner.commit(changes)
    }
    
    fn insert_account(&mut self, address: Address, account: AccountInfo) {
        self.inner.insert_account_info(address, account)
    }
    
    fn set_storage_at(&mut self, address: Address, slot: B256, val: B256) -> DatabaseResult<()> {
        self.inner.insert_account_storage(address, slot.into(), val.into())
    }
    
    fn insert_block_hash(&mut self, number: U256, hash: B256) {
        self.inner.block_hashes.insert(number, hash);
    }
    
    fn dump_state(
        &self,
        _at: revm::context::BlockEnv,
        _best_number: u64,
        _blocks: Vec<SerializableBlock>,
        _transactions: Vec<SerializableTransaction>,
        _historical_states: Option<SerializableHistoricalStates>,
    ) -> DatabaseResult<Option<SerializableState>> {
        // Simplified implementation for CacheDB
        Ok(None)
    }
    
    fn snapshot_state(&mut self) -> U256 {
        // Not supported for CacheDB
        U256::ZERO
    }
    
    fn revert_state(&mut self, _id: U256, _action: RevertStateSnapshotAction) -> bool {
        // Not supported for CacheDB
        false
    }
    
    fn current_state(&self) -> StateDb {
        // Simplified implementation
        StateDb::new(Box::new(self.inner.clone()) as Box<dyn MaybeFullDatabase>)
    }
}

/// Adapter for MemDb
#[derive(Debug)]
pub struct MemDbAdapter {
    pub inner: MemDb,
}

impl MemDbAdapter {
    pub fn new(inner: MemDb) -> Self {
        Self { inner }
    }
}

impl AnvilDatabase for MemDbAdapter {
    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, DatabaseError> {
        use revm::Database;
        self.inner.basic(address.1)
    }
    
    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        use revm::Database;
        self.inner.code_by_hash(code_hash)
    }
    
    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        use revm::Database;
        self.inner.storage(address.1, index)
    }
    
    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, DatabaseError> {
        use revm::Database;
        self.inner.block_hash(number)
    }
    
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.inner.commit(single_chain_changes)
    }
    
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, DatabaseError> {
        self.inner.basic_ref(address)
    }
    
    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        self.inner.code_by_hash_ref(code_hash)
    }
    
    fn storage_ref(&self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        self.inner.storage_ref(address, index)
    }
    
    fn block_hash_ref(&self, number: u64) -> Result<B256, DatabaseError> {
        self.inner.block_hash_ref(number)
    }
    
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, DatabaseError> {
        use revm::Database;
        self.inner.basic(address)
    }
    
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, DatabaseError> {
        use revm::Database;
        self.inner.code_by_hash(code_hash)
    }
    
    fn storage(&mut self, address: Address, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, DatabaseError> {
        use revm::Database;
        self.inner.storage(address, index)
    }
    
    fn block_hash(&mut self, number: u64) -> Result<B256, DatabaseError> {
        use revm::Database;
        self.inner.block_hash(number)
    }
    
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        use revm::DatabaseCommit;
        self.inner.commit(changes)
    }
    
    fn insert_account(&mut self, address: Address, account: AccountInfo) {
        self.inner.insert_account(address, account)
    }
    
    fn set_storage_at(&mut self, address: Address, slot: B256, val: B256) -> DatabaseResult<()> {
        self.inner.set_storage_at(address, slot, val)
    }
    
    fn insert_block_hash(&mut self, number: U256, hash: B256) {
        self.inner.insert_block_hash(number, hash)
    }
    
    fn dump_state(
        &self,
        at: revm::context::BlockEnv,
        best_number: u64,
        blocks: Vec<SerializableBlock>,
        transactions: Vec<SerializableTransaction>,
        historical_states: Option<SerializableHistoricalStates>,
    ) -> DatabaseResult<Option<SerializableState>> {
        self.inner.dump_state(at, best_number, blocks, transactions, historical_states)
    }
    
    fn snapshot_state(&mut self) -> U256 {
        self.inner.snapshot_state()
    }
    
    fn revert_state(&mut self, id: U256, action: RevertStateSnapshotAction) -> bool {
        self.inner.revert_state(id, action)
    }
    
    fn current_state(&self) -> StateDb {
        self.inner.current_state()
    }
    
    fn maybe_state_root(&self) -> Option<B256> {
        self.inner.maybe_state_root()
    }
}

// Similar adapters would be needed for ForkedDatabase and WrapDatabaseRef
// This approach avoids orphan rule violations by creating our own trait hierarchy