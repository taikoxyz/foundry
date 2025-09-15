# Upstream Coordination Plan: Multi-Chain Database Support

## 🎯 **OBJECTIVE**
Add multi-chain database trait implementations to upstream crates to resolve orphan rule violations in the Foundry multi-chain fork.

## 📋 **REQUIRED IMPLEMENTATIONS**

### **1. revm-private Crate Additions**
**Repository:** `revm-private` (private repository)  
**Required implementations:**

```rust
// File: src/database_impls.rs (new file)
use revm::database::{CacheDB, WrapDatabaseRef, DatabaseRef};
use crate::{MultiChainDatabase, MultiChainDatabaseCommit};
use alloy_primitives::{Address, B256, map::HashMap};
use revm::primitives::{ChainAddress, StorageKey, StorageValue};
use revm::state::{Account, AccountInfo};
use revm::bytecode::Bytecode;

// CacheDB MultiChainDatabase implementation
impl<T: DatabaseRef + Send + Sync + Clone + fmt::Debug> MultiChainDatabase for CacheDB<T> 
where T::Error: Into<DatabaseError>
{
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1).map_err(Into::into)
    }
    
    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash).map_err(Into::into)
    }
    
    fn storage_multi(&mut self, address: ChainAddress, index: StorageKey) -> Result<StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index).map_err(Into::into)
    }
    
    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number).map_err(Into::into)
    }
}

// CacheDB MultiChainDatabaseCommit implementation
impl<T: DatabaseRef + Send + Sync + Clone + fmt::Debug> MultiChainDatabaseCommit for CacheDB<T> 
where T::Error: Into<DatabaseError>
{
    fn commit_multi(&mut self, changes: HashMap<ChainAddress, Account>) {
        let single_chain_changes: HashMap<Address, Account> = changes
            .into_iter()
            .map(|(chain_addr, account)| (chain_addr.1, account))
            .collect();
        use revm::DatabaseCommit;
        self.commit(single_chain_changes)
    }
}

// WrapDatabaseRef MultiChainDatabase implementation
impl<T: DatabaseRef> MultiChainDatabase for WrapDatabaseRef<T> 
where T::Error: Into<DatabaseError>
{
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        self.0.basic_ref(address.1).map_err(Into::into)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.0.code_by_hash_ref(code_hash).map_err(Into::into)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: StorageKey) -> Result<StorageValue, Self::Error> {
        self.0.storage_ref(address.1, index).map_err(Into::into)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        self.0.block_hash_ref(number).map_err(Into::into)
    }
}
```

### **2. foundry-evm Crate Additions**
**Repository:** `https://github.com/foundry-rs/foundry`  
**File:** `crates/evm/evm/src/backend/mod.rs` or similar  
**Required implementations:**

```rust
// Add to existing file or create new database_multichain.rs
use revm::{
    context_interface::MultiChainDatabase,
    database_interface::MultiChainDatabaseCommit,
    primitives::ChainAddress,
    state::{Account, AccountInfo},
    bytecode::Bytecode,
    Database, DatabaseCommit,
};
use crate::backend::{MemDb, DatabaseError};
use crate::fork::database::ForkedDatabase;
use alloy_primitives::{Address, B256, map::HashMap};
use revm::primitives::{StorageKey, StorageValue};

// MemDb MultiChainDatabase implementation
impl MultiChainDatabase for MemDb {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: StorageKey) -> Result<StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number)
    }
}

// MemDb MultiChainDatabaseCommit implementation
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

// ForkedDatabase MultiChainDatabase implementation
impl MultiChainDatabase for ForkedDatabase {
    type Error = DatabaseError;

    fn basic_multi(&mut self, address: ChainAddress) -> Result<Option<AccountInfo>, Self::Error> {
        use revm::Database;
        self.basic(address.1)
    }

    fn code_by_hash_multi(&mut self, _chain_id: u64, code_hash: B256) -> Result<Bytecode, Self::Error> {
        use revm::Database;
        self.code_by_hash(code_hash)
    }

    fn storage_multi(&mut self, address: ChainAddress, index: StorageKey) -> Result<StorageValue, Self::Error> {
        use revm::Database;
        self.storage(address.1, index)
    }

    fn block_hash_multi(&mut self, _chain_id: u64, number: u64) -> Result<B256, Self::Error> {
        use revm::Database;
        self.block_hash(number)
    }
}

// ForkedDatabase MultiChainDatabaseCommit implementation
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
```

---

## 📞 **COORDINATION STEPS**

### **Step 1: Contact revm-private Maintainers**
Since `revm-private` is a private repository, you'll need to:
1. Contact the maintainers directly (likely through existing communication channels)
2. Explain the multi-chain architecture requirements
3. Propose adding the `CacheDB` and `WrapDatabaseRef` implementations
4. Provide the code above as a reference implementation

### **Step 2: Create foundry-evm Pull Request**
1. Fork the `foundry-rs/foundry` repository
2. Create a new branch: `feature/multi-chain-database-support`
3. Add the `MemDb` and `ForkedDatabase` implementations
4. Submit a pull request with detailed explanation

### **Step 3: Draft Pull Request Description**
```markdown
# Add Multi-Chain Database Support

## Summary
This PR adds `MultiChainDatabase` and `MultiChainDatabaseCommit` trait implementations 
for `MemDb` and `ForkedDatabase` to support multi-chain EVM architectures.

## Motivation
The multi-chain Anvil fork requires these implementations to avoid orphan rule 
violations (E0117) when implementing foreign traits for foreign types.

## Implementation Details
- Added `MultiChainDatabase` implementation for `MemDb`
- Added `MultiChainDatabaseCommit` implementation for `MemDb`  
- Added `MultiChainDatabase` implementation for `ForkedDatabase`
- Added `MultiChainDatabaseCommit` implementation for `ForkedDatabase`
- All implementations delegate to existing single-chain methods using `address.1`

## Testing
- [ ] Existing tests pass
- [ ] Multi-chain functionality verified
- [ ] No breaking changes to existing API

## Related Issues
Resolves orphan rule violations in multi-chain Foundry fork.
```

---

## 🔄 **ALTERNATIVE: TEMPORARY FORK APPROACH**

While waiting for upstream coordination, you can create temporary forks:

### **1. Fork revm-private**
- Create internal fork with the implementations
- Update `Cargo.toml` to use your fork:
```toml
[dependencies]
revm-private = { git = "https://github.com/your-org/revm-private", branch = "multi-chain-support" }
```

### **2. Fork foundry-evm** 
- Fork the foundry repository
- Add the implementations to your fork
- Update dependencies to use your fork

---

## 📋 **COMMUNICATION TEMPLATES**

### **Email/Message Template for revm-private**
```
Subject: Multi-Chain Database Support Request

Hi [Maintainer Name],

We're working on a multi-chain fork of Foundry/Anvil that requires implementing 
MultiChainDatabase and MultiChainDatabaseCommit traits for CacheDB and WrapDatabaseRef.

Currently, this creates orphan rule violations (E0117) since we can't implement 
foreign traits for foreign types in our codebase.

Would you be open to adding these implementations to revm-private? I have the 
complete working implementations ready and can provide them for review.

The implementations simply delegate to existing single-chain methods while 
extracting the address from ChainAddress tuples.

Please let me know if you'd like to discuss this further.

Best regards,
[Your Name]
```

### **GitHub Issue Template**
```markdown
**Feature Request: Multi-Chain Database Support**

**Problem:**
Multi-chain EVM architectures require implementing `MultiChainDatabase` and 
`MultiChainDatabaseCommit` traits for existing database types. This currently 
creates orphan rule violations.

**Proposed Solution:**
Add trait implementations for `MemDb` and `ForkedDatabase` in foundry-evm.

**Implementation:**
I have working implementations that delegate to existing methods. Happy to 
submit a PR if there's interest.

**Use Case:**
Enables multi-chain Anvil forks without orphan rule violations.
```

---

## ✅ **SUCCESS CRITERIA**

Once upstream coordination is complete:
1. Remove `crates/anvil/src/orphan_impls.rs`
2. Update imports to use upstream implementations
3. Verify clean compilation: `cargo build -p anvil`
4. All 7 orphan rule violations should be resolved

**This approach provides the cleanest long-term solution for the multi-chain architecture.**