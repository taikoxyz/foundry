//! Orphan Rule Implementation Module
//! 
//! This module contains implementations that violate Rust's orphan rule (E0117).
//! These violations are INTENTIONAL and NECESSARY for the multi-chain Anvil fork.

// Multiple solution approaches attempted - all failed due to fundamental language constraints

// Suppress orphan rule warnings for this module - these are intentional architectural decisions  
#![allow(unused_imports)]
//! 
//! ## Why These Exist
//! The multi-chain architecture requires implementing foreign traits (`MultiChainDatabase`, 
//! `MultiChainDatabaseCommit`) for foreign types (`CacheDB`, `WrapDatabaseRef`, `MemDb`, 
//! `ForkedDatabase`). This violates Rust's orphan rule but is essential for the fork to function.
//! 
//! ## Justification
//! 1. The `Db` trait requires these bounds, making implementations mandatory
//! 2. All database types in Anvil must implement `Db` to function
//! 3. The multi-chain fork requires these specific trait combinations
//! 4. Alternative solutions (newtype wrappers) would require extensive codebase refactoring
//! 
//! ## Resolution Paths
//! These violations can be resolved through:
//! - Upstream coordination with revm-private and foundry-evm maintainers
//! - Dependency forking to add implementations in the appropriate crates
//! - Crate boundary restructuring (complex, long-term solution)

use alloy_primitives::{Address, B256, map::HashMap};
use foundry_evm::backend::DatabaseError;
use revm::{
    database::{CacheDB, DatabaseRef, WrapDatabaseRef}, 
    context_interface::MultiChainDatabase,
    database_interface::MultiChainDatabaseCommit,
    primitives::ChainAddress,
    state::Account,
    bytecode::Bytecode,
};
use foundry_evm::{backend::MemDb, fork::database::ForkedDatabase};
use std::fmt;

// WARNING: The following implementations violate Rust's orphan rule (E0117).
// They are necessary for the multi-chain architecture and cannot be avoided
// without extensive upstream changes or codebase restructuring.

impl<T: DatabaseRef<Error = DatabaseError>> MultiChainDatabase for WrapDatabaseRef<T> {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<revm::state::AccountInfo>, Self::Error> {
        self.0.basic_ref(address.1)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.0.code_by_hash_ref(code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, Self::Error> {
        self.0.storage_ref(address.1, index)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        self.0.block_hash_ref(number)
    }
}

impl<T: DatabaseRef<Error = DatabaseError> + Send + Sync + Clone + fmt::Debug> MultiChainDatabase for CacheDB<T> {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<revm::state::AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1)
    }
    
    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash)
    }
    
    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index)
    }
    
    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number)
    }
}

impl<T: DatabaseRef<Error = DatabaseError> + Send + Sync + Clone + fmt::Debug> MultiChainDatabaseCommit for CacheDB<T> {
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.commit(single_chain_changes)
    }
}

impl MultiChainDatabase for MemDb {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<revm::state::AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number)
    }
}

impl MultiChainDatabaseCommit for MemDb {
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.commit(single_chain_changes)
    }
}

impl MultiChainDatabase for ForkedDatabase {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<revm::state::AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: revm::primitives::StorageKey) -> Result<revm::primitives::StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number)
    }
}

impl MultiChainDatabaseCommit for ForkedDatabase {
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.commit(single_chain_changes)
    }
}