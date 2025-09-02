use crate::eth::backend::db::{
    Db, MaybeForkedDatabase, MaybeFullDatabase, SerializableAccountRecord, SerializableBlock,
    SerializableHistoricalStates, SerializableState, SerializableTransaction, StateDb,
};
use alloy_primitives::{Address, B256, U256, map::HashMap};
use alloy_rpc_types::BlockId;
use foundry_evm::{
    backend::{
        BlockchainDb, DatabaseError, DatabaseResult, RevertStateSnapshotAction, StateSnapshot,
    },
    fork::database::ForkDbStateSnapshot,
};
use revm::{
    context::BlockEnv,
    database::{Database, DatabaseRef, DbAccount},
    state::AccountInfo,
};

pub use foundry_evm::fork::database::ForkedDatabase;

impl Db for ForkedDatabase {
    fn insert_account(&mut self, address: Address, account: AccountInfo) {
        self.database_mut().insert_account(address, account)
    }

    fn set_storage_at(&mut self, address: Address, slot: B256, val: B256) -> DatabaseResult<()> {
        // this ensures the account is loaded first
        let _ = Database::basic(self, address)?;
        self.database_mut().set_storage_at(address, slot, val)
    }

    fn insert_block_hash(&mut self, number: U256, hash: B256, chain_id: u64) {
        self.inner().block_hashes().write().insert((chain_id, number), hash);
    }

    fn dump_state(
        &self,
        at: BlockEnv,
        best_number: u64,
        blocks: Vec<SerializableBlock>,
        transactions: Vec<SerializableTransaction>,
        historical_states: Option<SerializableHistoricalStates>,
    ) -> DatabaseResult<Option<SerializableState>> {
        let mut db = self.database().clone();
        let accounts = self
            .database()
            .cache
            .accounts
            .clone()
            .into_iter()
            .map(|(k, v)| -> DatabaseResult<_> {
                let code = if let Some(code) = v.info.code {
                    code
                } else {
                    db.code_by_hash(v.info.code_hash)?
                };
                Ok((
                    k,
                    SerializableAccountRecord {
                        nonce: v.info.nonce,
                        balance: v.info.balance,
                        code: code.original_bytes(),
                        storage: v.storage.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
                    },
                ))
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(SerializableState {
            block: Some(at),
            accounts,
            best_block_number: Some(best_number),
            blocks,
            transactions,
            historical_states,
        }))
    }

    fn snapshot_state(&mut self) -> U256 {
        self.insert_state_snapshot()
    }

    fn revert_state(&mut self, id: U256, action: RevertStateSnapshotAction) -> bool {
        self.revert_state_snapshot(id, action)
    }

    fn current_state(&self) -> StateDb {
        StateDb::new(self.create_state_snapshot())
    }
}

impl MaybeFullDatabase for ForkedDatabase {
    fn as_dyn(&self) -> &dyn DatabaseRef<Error = DatabaseError> {
        self
    }

    fn maybe_as_full_db(&self) -> Option<&HashMap<Address, DbAccount>> {
        Some(&self.database().cache.accounts)
    }

    fn clear_into_state_snapshot(&mut self) -> StateSnapshot {
        let db = self.inner().db();
        let accounts_lock = std::mem::take(&mut *db.accounts.write());
        let storage_lock = std::mem::take(&mut *db.storage.write());
        let block_hashes_lock = std::mem::take(&mut *db.block_hashes.write());

        // Convert from chain-aware to non-chain-aware types
        let mut accounts = HashMap::default();
        for ((_, addr), info) in accounts_lock {
            accounts.insert(addr, info);
        }

        let mut storage = HashMap::default();
        for ((_, addr), store) in storage_lock {
            storage.insert(addr, store);
        }

        let mut block_hashes = HashMap::default();
        for ((_, num), hash) in block_hashes_lock {
            block_hashes.insert(num, hash);
        }

        StateSnapshot { accounts, storage, block_hashes }
    }

    fn read_as_state_snapshot(&self) -> StateSnapshot {
        let db = self.inner().db();

        // Convert from chain-aware to non-chain-aware types
        let accounts_lock = db.accounts.read();
        let mut accounts = HashMap::default();
        for ((_, addr), info) in accounts_lock.iter() {
            accounts.insert(*addr, info.clone());
        }

        let storage_lock = db.storage.read();
        let mut storage = HashMap::default();
        for ((_, addr), store) in storage_lock.iter() {
            storage.insert(*addr, store.clone());
        }

        let block_hashes_lock = db.block_hashes.read();
        let mut block_hashes = HashMap::default();
        for ((_, num), hash) in block_hashes_lock.iter() {
            block_hashes.insert(*num, *hash);
        }

        StateSnapshot { accounts, storage, block_hashes }
    }

    fn clear(&mut self) {
        self.flush_cache();
        self.clear_into_state_snapshot();
    }

    fn init_from_state_snapshot(&mut self, state_snapshot: StateSnapshot) {
        let db = self.inner().db();
        let StateSnapshot { accounts, storage, block_hashes } = state_snapshot;

        // Convert from non-chain-aware to chain-aware types
        let mut chain_accounts = HashMap::default();
        for (addr, info) in accounts {
            // Use default chain_id of 1 for anvil
            chain_accounts.insert((1u64, addr), info);
        }
        *db.accounts.write() = chain_accounts;

        let mut chain_storage = HashMap::default();
        for (addr, store) in storage {
            // Use default chain_id of 1 for anvil
            chain_storage.insert((1u64, addr), store);
        }
        *db.storage.write() = chain_storage;

        let mut chain_block_hashes = HashMap::default();
        for (num, hash) in block_hashes {
            // Use default chain_id of 1 for anvil
            chain_block_hashes.insert((1u64, num), hash);
        }
        *db.block_hashes.write() = chain_block_hashes;
    }
}

impl MaybeFullDatabase for ForkDbStateSnapshot {
    fn as_dyn(&self) -> &dyn DatabaseRef<Error = DatabaseError> {
        self
    }

    fn maybe_as_full_db(&self) -> Option<&HashMap<Address, DbAccount>> {
        Some(&self.local.cache.accounts)
    }

    fn clear_into_state_snapshot(&mut self) -> StateSnapshot {
        std::mem::take(&mut self.state_snapshot)
    }

    fn read_as_state_snapshot(&self) -> StateSnapshot {
        self.state_snapshot.clone()
    }

    fn clear(&mut self) {
        std::mem::take(&mut self.state_snapshot);
        self.local.clear()
    }

    fn init_from_state_snapshot(&mut self, state_snapshot: StateSnapshot) {
        self.state_snapshot = state_snapshot;
    }
}

impl MaybeForkedDatabase for ForkedDatabase {
    fn maybe_reset(&mut self, url: Option<String>, block_number: BlockId) -> Result<(), String> {
        self.reset(url, block_number)
    }

    fn maybe_flush_cache(&self) -> Result<(), String> {
        self.flush_cache();
        Ok(())
    }

    fn maybe_inner(&self) -> Result<&BlockchainDb, String> {
        Ok(self.inner())
    }
}
